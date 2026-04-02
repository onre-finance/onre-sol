use crate::constants::seeds;
use crate::instructions::buffer::accounts::{
    BufferAccrualAccountsBumps, __client_accounts_buffer_accrual_accounts,
    __cpi_client_accounts_buffer_accrual_accounts,
};
use crate::instructions::buffer::{
    accrue_buffer::{accrue_buffer_from_accounts, store_buffer_post_supply},
    BufferAccrualAccounts,
};
use crate::instructions::market_info::{load_main_offer, refresh_market_stats_pda};
use crate::instructions::redemption::{
    execute_redemption_operations, process_redemption_core, ExecuteRedemptionOpsParams,
    RedemptionFeeVaultAuthority, RedemptionOffer, RedemptionRequest,
};
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::{
    load_or_init_pda_account, load_pda_account, program_controls_mint, store_pda_account,
    PdaAccountInit,
};
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::{
    associated_token::{get_associated_token_address_with_program_id, AssociatedToken},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

/// Event emitted when a redemption request is fulfilled (fully or partially)
///
/// Provides transparency for tracking redemption fulfillment and token exchange details.
#[event]
pub struct RedemptionRequestFulfilledEvent {
    /// The PDA address of the fulfilled redemption request
    pub redemption_request_pda: Pubkey,
    /// Reference to the redemption offer pda
    pub redemption_offer_pda: Pubkey,
    /// User who created the redemption request
    pub redeemer: Pubkey,
    /// Net amount of token_in tokens burned/transferred in this fulfillment call (after fees)
    pub token_in_net_amount: u64,
    /// Fee amount deducted from token_in in this fulfillment call
    pub token_in_fee_amount: u64,
    /// Amount of token_out tokens received by the user in this fulfillment call
    pub token_out_amount: u64,
    /// Current price used for the redemption
    pub current_price: u64,
    /// Amount of token_in fulfilled in this call (before fee deduction)
    pub fulfilled_amount: u64,
    /// Cumulative token_in amount fulfilled across all calls for this request
    pub total_fulfilled_amount: u64,
    /// Whether the request is now fully settled (account closed)
    pub is_fully_fulfilled: bool,
}

/// Account structure for fulfilling a redemption request
///
/// This struct defines the accounts required to fulfill a redemption request,
/// handling token burning/transfer for token_in (typically ONyc) and minting/transfer
/// for token_out (typically stablecoins like USDC).
#[derive(Accounts)]
pub struct FulfillRedemptionRequest<'info> {
    /// Program state account containing redemption_admin and boss authorization
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ FulfillRedemptionRequestErrorCode::InvalidBoss,
        constraint = !state.is_killed @ FulfillRedemptionRequestErrorCode::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// The boss account that may receive tokens when program lacks mint authority
    /// CHECK: Account validation is enforced through state account constraint
    pub boss: UncheckedAccount<'info>,

    /// The underlying offer that defines pricing
    /// CHECK: offer address is validated through redemption_offer constraint
    pub offer: AccountLoader<'info, Offer>,

    /// CHECK: Validated and stored manually in instruction logic.
    #[account(mut)]
    pub redemption_offer: UncheckedAccount<'info>,

    /// The redemption request account to fulfill (partially or fully)
    ///
    /// The account is only closed when the request is fully fulfilled
    /// (fulfilled_amount == amount). For partial fulfillments the account
    /// remains open so further fulfillment calls can be made.
    #[account(
        mut,
        seeds = [
            seeds::REDEMPTION_REQUEST,
            redemption_request.offer.as_ref(),
            redemption_request.request_id.to_le_bytes().as_ref()
        ],
        bump = redemption_request.bump,
        constraint = redemption_request.offer == redemption_offer.key()
            @ FulfillRedemptionRequestErrorCode::OfferMismatch
    )]
    pub redemption_request: Box<Account<'info, RedemptionRequest>>,

    /// Program-derived redemption vault authority that controls token operations
    ///
    /// This PDA manages token transfers and burning operations.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY],
        bump
    )]
    pub redemption_vault_authority: UncheckedAccount<'info>,

    /// Redemption vault account for token_in (to receive tokens for burning or storage)
    ///
    /// Used as intermediate account when burning token_in or as permanent storage
    /// when program lacks mint authority.
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_in_program
    )]
    pub vault_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Redemption vault account for token_out distribution when using transfer mechanism
    ///
    /// Source of output tokens when the program lacks mint authority
    /// and must transfer from pre-funded vault instead of minting.
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_out_program
    )]
    pub vault_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Input token mint (typically ONyc)
    ///
    /// Must be mutable to allow burning operations when program has mint authority.
    #[account(mut)]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for input token operations
    pub token_in_program: Interface<'info, TokenInterface>,

    /// Output token mint (typically stablecoin like USDC)
    ///
    /// Must be mutable to allow minting operations when program has mint authority.
    #[account(mut)]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for output token operations
    pub token_out_program: Interface<'info, TokenInterface>,

    /// User's output token account (destination for redeemed tokens)
    ///
    /// Created automatically if it doesn't exist.
    #[account(
        init_if_needed,
        payer = redemption_admin,
        associated_token::mint = token_out_mint,
        associated_token::authority = redeemer,
        associated_token::token_program = token_out_program
    )]
    pub user_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Boss's input token account for receiving net tokens when program lacks mint authority
    ///
    /// Only used when program doesn't have mint authority of token_in.
    #[account(
        init_if_needed,
        payer = redemption_admin,
        associated_token::mint = token_in_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_in_program
    )]
    pub boss_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Global fee vault authority PDA — created on first fulfillment if not yet initialized
    /// CHECK: PDA address is validated and the account is initialized/loaded manually.
    #[account(mut)]
    pub redemption_fee_vault_authority: UncheckedAccount<'info>,

    /// The account that should receive fees.
    /// Must equal `redemption_fee_vault_authority.fee_destination` when set,
    /// or the vault authority PDA itself when `fee_destination` is default.
    /// CHECK: validated in function body against stored fee_destination
    pub fee_destination: UncheckedAccount<'info>,

    /// ATA of `fee_destination` for token_in — receives the fee portion
    #[account(
        init_if_needed,
        payer = redemption_admin,
        associated_token::mint = token_in_mint,
        associated_token::authority = fee_destination,
        associated_token::token_program = token_in_program
    )]
    pub fee_destination_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Program-derived mint authority for direct token minting
    ///
    /// Used when the program has mint authority and can mint token_out directly.
    /// CHECK: PDA derivation is validated through seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,

    /// The user who created the redemption request
    /// CHECK: Validated against redemption_request.redeemer
    #[account(constraint = redeemer.key() == redemption_request.redeemer
        @ FulfillRedemptionRequestErrorCode::InvalidRedeemer)]
    pub redeemer: UncheckedAccount<'info>,

    /// Redemption admin must sign to authorize fulfillment
    #[account(
        mut,
        constraint = redemption_admin.key() == state.redemption_admin
            @ FulfillRedemptionRequestErrorCode::Unauthorized
    )]
    pub redemption_admin: Signer<'info>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program required for account creation
    pub system_program: Program<'info, System>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub offer_vault_authority: UncheckedAccount<'info>,

    /// CHECK: Address is validated against the canonical ATA derivation.
    #[account(
        constraint = offer_vault_onyc_account.key()
            == get_associated_token_address_with_program_id(
                &offer_vault_authority.key(),
                &token_in_mint.key(),
                &token_in_program.key(),
            ) @ crate::instructions::market_info::GetCirculatingSupplyErrorCode::InvalidVaultAccount
    )]
    pub offer_vault_onyc_account: UncheckedAccount<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against state.main_offer.
    pub main_offer: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct FulfillRedemptionRequest<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ FulfillRedemptionRequestErrorCode::InvalidBoss,
        constraint = !state.is_killed @ FulfillRedemptionRequestErrorCode::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// CHECK: Account validation is enforced through state account constraint
    pub boss: UncheckedAccount<'info>,

    /// CHECK: offer address is validated through redemption_offer constraint
    pub offer: AccountLoader<'info, Offer>,

    /// CHECK: Validated and stored manually in instruction logic.
    #[account(mut)]
    pub redemption_offer: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [
            seeds::REDEMPTION_REQUEST,
            redemption_request.offer.as_ref(),
            redemption_request.request_id.to_le_bytes().as_ref()
        ],
        bump = redemption_request.bump,
        constraint = redemption_request.offer == redemption_offer.key()
            @ FulfillRedemptionRequestErrorCode::OfferMismatch
    )]
    pub redemption_request: Box<Account<'info, RedemptionRequest>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY],
        bump
    )]
    pub redemption_vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_in_program
    )]
    pub vault_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_out_program
    )]
    pub vault_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_in_program: Interface<'info, TokenInterface>,

    #[account(mut)]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_out_program: Interface<'info, TokenInterface>,

    #[account(
        init_if_needed,
        payer = redemption_admin,
        associated_token::mint = token_out_mint,
        associated_token::authority = redeemer,
        associated_token::token_program = token_out_program
    )]
    pub user_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = redemption_admin,
        associated_token::mint = token_in_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_in_program
    )]
    pub boss_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Global fee vault authority PDA — created on first fulfillment if not yet initialized
    /// CHECK: PDA address is validated and the account is initialized/loaded manually.
    #[account(mut)]
    pub redemption_fee_vault_authority: UncheckedAccount<'info>,

    /// The account that should receive fees.
    /// Must equal `redemption_fee_vault_authority.fee_destination` when set,
    /// or the vault authority PDA itself when `fee_destination` is default.
    /// CHECK: validated in function body against stored fee_destination
    pub fee_destination: UncheckedAccount<'info>,

    /// ATA of `fee_destination` for token_in — receives the fee portion
    #[account(
        init_if_needed,
        payer = redemption_admin,
        associated_token::mint = token_in_mint,
        associated_token::authority = fee_destination,
        associated_token::token_program = token_in_program
    )]
    pub fee_destination_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated through seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,

    /// CHECK: Validated against redemption_request.redeemer
    #[account(constraint = redeemer.key() == redemption_request.redeemer
        @ FulfillRedemptionRequestErrorCode::InvalidRedeemer)]
    pub redeemer: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = redemption_admin.key() == state.redemption_admin
            @ FulfillRedemptionRequestErrorCode::Unauthorized
    )]
    pub redemption_admin: Signer<'info>,

    pub buffer_accounts: BufferAccrualAccounts<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub offer_vault_authority: UncheckedAccount<'info>,

    /// CHECK: Address is validated against the canonical ATA derivation.
    #[account(
        constraint = offer_vault_onyc_account.key()
            == get_associated_token_address_with_program_id(
                &offer_vault_authority.key(),
                &token_in_mint.key(),
                &token_in_program.key(),
            ) @ crate::instructions::market_info::GetCirculatingSupplyErrorCode::InvalidVaultAccount
    )]
    pub offer_vault_onyc_account: UncheckedAccount<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against state.main_offer.
    pub main_offer: UncheckedAccount<'info>,
}

/// Fulfills a redemption request, either fully or partially
///
/// This instruction processes `amount` tokens from a pending redemption request.
/// Calling with `amount` less than the remaining unfulfilled balance is a partial
/// fulfillment — the account stays open and further calls can be made until the
/// request is fully settled.  Calling with the exact remaining balance (or passing
/// `request.amount - request.fulfilled_amount`) closes the account and returns rent.
///
/// # Arguments
/// * `ctx`    - The instruction context containing validated accounts
/// * `amount` - Amount of token_in to process in this call. Must be > 0 and ≤ remaining
///              unfulfilled balance (`request.amount - request.fulfilled_amount`).
///
/// # Returns
/// * `Ok(())` - If the (partial) redemption is successfully processed
/// * `Err(_)` - If validation fails or token operations fail
///
/// # Access Control
/// - Only redemption_admin can fulfill redemptions
/// - Kill switch prevents fulfillment when activated
///
/// # Effects
/// - Processes `amount` tokens at the current NAV price
/// - Updates `request.fulfilled_amount`; closes account when fully settled
/// - Decrements `redemption_offer.requested_redemptions` by `amount`
/// - Increments `redemption_offer.executed_redemptions` by `amount`
/// - Burns or transfers token_in based on mint authority
/// - Mints or transfers token_out to user
///
/// # Events
/// * `RedemptionRequestFulfilledEvent` - Emitted with fulfillment details
pub fn fulfill_redemption_request(
    ctx: Context<FulfillRedemptionRequest>,
    amount: u64,
) -> Result<()> {
    let mut redemption_offer = load_redemption_offer(
        ctx.program_id,
        &ctx.accounts.offer,
        &ctx.accounts.redemption_offer,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
    )?;
    let mut redemption_fee_vault_authority = load_redemption_fee_vault_authority(
        ctx.program_id,
        &ctx.accounts.redemption_fee_vault_authority,
        &ctx.accounts.redemption_admin.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
    )?;
    execute_fulfill_redemption_request(
        ExecuteFulfillRedemptionRequestParams {
            program_id: ctx.program_id,
            state: &ctx.accounts.state,
            offer: &ctx.accounts.offer,
            redemption_offer_account: &ctx.accounts.redemption_offer,
            redemption_offer: &mut redemption_offer,
            redemption_request: &mut ctx.accounts.redemption_request,
            redemption_fee_vault_authority_account: &ctx.accounts.redemption_fee_vault_authority,
            redemption_fee_vault_authority: &mut redemption_fee_vault_authority,
            vault_token_in_account: &ctx.accounts.vault_token_in_account,
            vault_token_out_account: &ctx.accounts.vault_token_out_account,
            token_in_mint: &mut ctx.accounts.token_in_mint,
            token_in_program: &ctx.accounts.token_in_program,
            token_out_mint: &ctx.accounts.token_out_mint,
            token_out_program: &ctx.accounts.token_out_program,
            user_token_out_account: &ctx.accounts.user_token_out_account,
            boss_token_in_account: &ctx.accounts.boss_token_in_account,
            fee_destination: &ctx.accounts.fee_destination,
            fee_destination_token_in_account: &ctx.accounts.fee_destination_token_in_account,
            mint_authority: &ctx.accounts.mint_authority,
            offer_vault_onyc_account: &ctx.accounts.offer_vault_onyc_account,
            redemption_vault_authority: &ctx.accounts.redemption_vault_authority,
            redemption_vault_authority_bump: ctx.bumps.redemption_vault_authority,
            mint_authority_bump: ctx.bumps.mint_authority,
            market_stats: &ctx.accounts.market_stats,
            main_offer: &ctx.accounts.main_offer,
            redeemer: &ctx.accounts.redeemer,
            redemption_admin: &ctx.accounts.redemption_admin,
            system_program: &ctx.accounts.system_program,
            buffer_accounts: None,
        },
        amount,
    )
}

pub fn fulfill_redemption_request(
    ctx: Context<FulfillRedemptionRequest>,
    amount: u64,
) -> Result<()> {
    let mut redemption_offer = load_redemption_offer(
        ctx.program_id,
        &ctx.accounts.offer,
        &ctx.accounts.redemption_offer,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
    )?;
    let mut redemption_fee_vault_authority = load_redemption_fee_vault_authority(
        ctx.program_id,
        &ctx.accounts.redemption_fee_vault_authority,
        &ctx.accounts.redemption_admin.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
    )?;
    execute_fulfill_redemption_request(
        ExecuteFulfillRedemptionRequestParams {
            program_id: ctx.program_id,
            state: &ctx.accounts.state,
            offer: &ctx.accounts.offer,
            redemption_offer_account: &ctx.accounts.redemption_offer,
            redemption_offer: &mut redemption_offer,
            redemption_request: &mut ctx.accounts.redemption_request,
            redemption_fee_vault_authority_account: &ctx.accounts.redemption_fee_vault_authority,
            redemption_fee_vault_authority: &mut redemption_fee_vault_authority,
            vault_token_in_account: &ctx.accounts.vault_token_in_account,
            vault_token_out_account: &ctx.accounts.vault_token_out_account,
            token_in_mint: &mut ctx.accounts.token_in_mint,
            token_in_program: &ctx.accounts.token_in_program,
            token_out_mint: &ctx.accounts.token_out_mint,
            token_out_program: &ctx.accounts.token_out_program,
            user_token_out_account: &ctx.accounts.user_token_out_account,
            boss_token_in_account: &ctx.accounts.boss_token_in_account,
            fee_destination: &ctx.accounts.fee_destination,
            fee_destination_token_in_account: &ctx.accounts.fee_destination_token_in_account,
            mint_authority: &ctx.accounts.mint_authority,
            offer_vault_onyc_account: &ctx.accounts.offer_vault_onyc_account,
            redemption_vault_authority: &ctx.accounts.redemption_vault_authority,
            redemption_vault_authority_bump: ctx.bumps.redemption_vault_authority,
            mint_authority_bump: ctx.bumps.mint_authority,
            market_stats: &ctx.accounts.market_stats,
            main_offer: &ctx.accounts.main_offer,
            redeemer: &ctx.accounts.redeemer,
            redemption_admin: &ctx.accounts.redemption_admin,
            system_program: &ctx.accounts.system_program,
            buffer_accounts: Some(&ctx.accounts.buffer_accounts),
        },
        amount,
    )
}

struct ExecuteFulfillRedemptionRequestParams<'a, 'info> {
    program_id: &'a Pubkey,
    state: &'a Account<'info, State>,
    offer: &'a AccountLoader<'info, Offer>,
    redemption_offer_account: &'a UncheckedAccount<'info>,
    redemption_offer: &'a mut RedemptionOffer,
    redemption_request: &'a mut Account<'info, RedemptionRequest>,
    redemption_fee_vault_authority_account: &'a UncheckedAccount<'info>,
    redemption_fee_vault_authority: &'a mut RedemptionFeeVaultAuthority,
    vault_token_in_account: &'a InterfaceAccount<'info, TokenAccount>,
    vault_token_out_account: &'a InterfaceAccount<'info, TokenAccount>,
    token_in_mint: &'a mut InterfaceAccount<'info, Mint>,
    token_in_program: &'a Interface<'info, TokenInterface>,
    token_out_mint: &'a InterfaceAccount<'info, Mint>,
    token_out_program: &'a Interface<'info, TokenInterface>,
    user_token_out_account: &'a InterfaceAccount<'info, TokenAccount>,
    boss_token_in_account: &'a InterfaceAccount<'info, TokenAccount>,
    fee_destination: &'a UncheckedAccount<'info>,
    fee_destination_token_in_account: &'a InterfaceAccount<'info, TokenAccount>,
    mint_authority: &'a UncheckedAccount<'info>,
    offer_vault_onyc_account: &'a UncheckedAccount<'info>,
    redemption_vault_authority: &'a UncheckedAccount<'info>,
    redemption_vault_authority_bump: u8,
    mint_authority_bump: u8,
    market_stats: &'a UncheckedAccount<'info>,
    main_offer: &'a UncheckedAccount<'info>,
    redeemer: &'a UncheckedAccount<'info>,
    redemption_admin: &'a Signer<'info>,
    system_program: &'a Program<'info, System>,
    buffer_accounts: Option<&'a BufferAccrualAccounts<'info>>,
}

impl PdaAccountInit for RedemptionFeeVaultAuthority {
    fn pda_seed_prefixes() -> &'static [&'static [u8]] {
        &[seeds::REDEMPTION_FEE_VAULT_AUTHORITY]
    }

    fn init_space() -> usize {
        8 + RedemptionFeeVaultAuthority::INIT_SPACE
    }

    fn init_value(bump: u8) -> Self {
        Self {
            fee_destination: Pubkey::default(),
            bump,
            reserved: [0; 31],
        }
    }

    fn invalid_owner_error() -> Error {
        error!(FulfillRedemptionRequestErrorCode::InvalidRedemptionFeeVaultAuthorityOwner)
    }

    fn invalid_data_error() -> Error {
        error!(FulfillRedemptionRequestErrorCode::InvalidRedemptionFeeVaultAuthorityData)
    }
}

fn load_redemption_offer<'info>(
    program_id: &Pubkey,
    offer: &AccountLoader<'info, Offer>,
    redemption_offer_account: &UncheckedAccount<'info>,
    token_in_mint: &InterfaceAccount<'info, Mint>,
    token_out_mint: &InterfaceAccount<'info, Mint>,
) -> Result<RedemptionOffer> {
    let redemption_offer: RedemptionOffer = load_pda_account(
        &redemption_offer_account.to_account_info(),
        program_id,
        FulfillRedemptionRequestErrorCode::InvalidRedemptionOfferOwner.into(),
        FulfillRedemptionRequestErrorCode::InvalidRedemptionOfferData.into(),
    )?;
    let (expected_redemption_offer, _) = Pubkey::find_program_address(
        &[
            seeds::REDEMPTION_OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref(),
        ],
        program_id,
    );

    require_keys_eq!(
        redemption_offer_account.key(),
        expected_redemption_offer,
        FulfillRedemptionRequestErrorCode::OfferMismatch
    );
    require_keys_eq!(
        redemption_offer.offer,
        offer.key(),
        FulfillRedemptionRequestErrorCode::OfferMismatch
    );
    require_keys_eq!(
        token_in_mint.key(),
        redemption_offer.token_in_mint,
        FulfillRedemptionRequestErrorCode::InvalidTokenInMint
    );
    require_keys_eq!(
        token_out_mint.key(),
        redemption_offer.token_out_mint,
        FulfillRedemptionRequestErrorCode::InvalidTokenOutMint
    );

    Ok(redemption_offer)
}

fn load_redemption_fee_vault_authority<'info>(
    program_id: &Pubkey,
    redemption_fee_vault_authority_account: &UncheckedAccount<'info>,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
) -> Result<RedemptionFeeVaultAuthority> {
    let (expected_redemption_fee_vault_authority, bump) =
        Pubkey::find_program_address(&[seeds::REDEMPTION_FEE_VAULT_AUTHORITY], program_id);

    require_keys_eq!(
        redemption_fee_vault_authority_account.key(),
        expected_redemption_fee_vault_authority,
        FulfillRedemptionRequestErrorCode::InvalidRedemptionFeeVaultAuthority
    );

    load_or_init_pda_account::<RedemptionFeeVaultAuthority>(
        &redemption_fee_vault_authority_account.to_account_info(),
        payer,
        system_program,
        program_id,
        bump,
    )
}

fn execute_fulfill_redemption_request(
    mut params: ExecuteFulfillRedemptionRequestParams,
    amount: u64,
) -> Result<()> {
    // Validate amount
    require!(amount > 0, FulfillRedemptionRequestErrorCode::InvalidAmount);

    let redemption_request = &params.redemption_request;
    let remaining = redemption_request
        .amount
        .checked_sub(redemption_request.fulfilled_amount)
        .ok_or(FulfillRedemptionRequestErrorCode::ArithmeticUnderflow)?;

    require!(
        amount <= remaining,
        FulfillRedemptionRequestErrorCode::AmountExceedsRemaining
    );

    // Validate fee_destination against stored value in the vault authority
    let expected_fee_destination = {
        let stored = params.redemption_fee_vault_authority.fee_destination;
        if stored == Pubkey::default() {
            params.redemption_fee_vault_authority_account.key()
        } else {
            stored
        }
    };
    require!(
        params.fee_destination.key() == expected_fee_destination,
        FulfillRedemptionRequestErrorCode::InvalidFeeDestination
    );

    // Use shared core processing logic for redemption
    let offer = params.offer.load()?;
    let result = process_redemption_core(
        &offer,
        amount,
        params.token_in_mint,
        params.token_out_mint,
        params.redemption_offer.fee_basis_points,
    )?;
    let price = result.price;
    let token_in_net_amount = result.token_in_net_amount;
    let token_in_fee_amount = result.token_in_fee_amount;
    let token_out_amount = result.token_out_amount;
    let should_refresh_market_stats = params.token_in_mint.key() == params.state.onyc_mint
        && program_controls_mint(
            params.token_in_mint,
            &params.mint_authority.to_account_info(),
        );
    let accrual = if let Some(buffer_accounts) = params
        .buffer_accounts
        .filter(|accounts| should_refresh_market_stats && accounts.is_initialized())
    {
        Some(accrue_buffer_from_accounts(
            params.program_id,
            params.state,
            buffer_accounts,
            &offer,
            params.token_in_mint,
            params.mint_authority.to_account_info(),
            params.mint_authority_bump,
            params.token_in_program,
        )?)
    } else {
        None
    };

    execute_redemption_operations(ExecuteRedemptionOpsParams {
        token_in_program: params.token_in_program,
        token_out_program: params.token_out_program,
        token_in_mint: params.token_in_mint,
        token_in_net_amount,
        token_in_fee_amount,
        vault_token_in_account: params.vault_token_in_account,
        boss_token_in_account: params.boss_token_in_account,
        fee_destination_token_in_account: params.fee_destination_token_in_account,
        redemption_vault_authority: &params.redemption_vault_authority.to_account_info(),
        redemption_vault_authority_bump: params.redemption_vault_authority_bump,
        token_out_mint: params.token_out_mint,
        token_out_amount,
        vault_token_out_account: params.vault_token_out_account,
        user_token_out_account: params.user_token_out_account,
        mint_authority_pda: &params.mint_authority.to_account_info(),
        mint_authority_bump: params.mint_authority_bump,
        token_out_max_supply: 0,
    })?;

    if let Some(accrual) = accrual {
        let post_burn_supply = accrual
            .post_accrual_supply
            .checked_sub(token_in_net_amount)
            .ok_or(crate::instructions::buffer::BufferErrorCode::MathOverflow)?;
        store_buffer_post_supply(
            params
                .buffer_accounts
                .expect("accrual implies buffer accounts"),
            post_burn_supply,
            accrual.timestamp,
        )?;
    }

    if should_refresh_market_stats {
        let main_offer = load_main_offer(
            params.program_id,
            &params.main_offer.to_account_info(),
            params.state,
        )?;
        params.token_in_mint.reload()?;
        refresh_market_stats_pda(
            &main_offer,
            params.token_in_mint,
            &params.offer_vault_onyc_account.to_account_info(),
            params.token_in_program,
            &params.market_stats.to_account_info(),
            &params.redemption_admin.to_account_info(),
            &params.system_program.to_account_info(),
            params.program_id,
        )?;
    }

    // Update fulfilled amount on the request
    let new_fulfilled_amount = params
        .redemption_request
        .fulfilled_amount
        .checked_add(amount)
        .ok_or(FulfillRedemptionRequestErrorCode::ArithmeticOverflow)?;
    params.redemption_request.fulfilled_amount = new_fulfilled_amount;

    let is_fully_fulfilled = new_fulfilled_amount == params.redemption_request.amount;

    // Update offer-level counters
    let redemption_offer = &mut params.redemption_offer;
    redemption_offer.executed_redemptions = redemption_offer
        .executed_redemptions
        .checked_add(amount as u128)
        .ok_or(FulfillRedemptionRequestErrorCode::ArithmeticOverflow)?;

    redemption_offer.requested_redemptions = redemption_offer
        .requested_redemptions
        .checked_sub(amount as u128)
        .ok_or(FulfillRedemptionRequestErrorCode::ArithmeticUnderflow)?;

    msg!(
        "Redemption request {}: fulfilled {} (net={}, fee={}), token_out={}, price={}, redeemer={}, total_fulfilled={}/{}, fully_fulfilled={}",
        params.redemption_request.key(),
        amount,
        token_in_net_amount,
        token_in_fee_amount,
        token_out_amount,
        price,
        params.redeemer.key(),
        new_fulfilled_amount,
        params.redemption_request.amount,
        is_fully_fulfilled,
    );

    emit!(RedemptionRequestFulfilledEvent {
        redemption_request_pda: params.redemption_request.key(),
        redemption_offer_pda: params.redemption_offer_account.key(),
        redeemer: params.redeemer.key(),
        token_in_net_amount,
        token_in_fee_amount,
        token_out_amount,
        current_price: price,
        fulfilled_amount: amount,
        total_fulfilled_amount: new_fulfilled_amount,
        is_fully_fulfilled,
    });

    store_pda_account(
        &params.redemption_offer_account.to_account_info(),
        params.redemption_offer,
    )?;
    store_pda_account(
        &params
            .redemption_fee_vault_authority_account
            .to_account_info(),
        params.redemption_fee_vault_authority,
    )?;

    // Close the request account only when fully settled; rent goes to redemption_admin
    if is_fully_fulfilled {
        params
            .redemption_request
            .close(params.redemption_admin.to_account_info())?;
    }

    Ok(())
}

/// Error codes for redemption fulfillment operations
#[error_code]
pub enum FulfillRedemptionRequestErrorCode {
    /// Caller is not authorized (redemption_admin mismatch)
    #[msg("Unauthorized: redemption_admin signature required")]
    Unauthorized,

    /// The boss account does not match the one stored in program state
    #[msg("Invalid boss account")]
    InvalidBoss,

    /// The program kill switch is activated
    #[msg("Kill switch is activated")]
    KillSwitchActivated,

    /// Redemption offer mismatch
    #[msg("Redemption offer does not match request")]
    OfferMismatch,

    /// Offer mint configuration mismatch
    #[msg("Offer mints do not match redemption offer (inverted) mints")]
    OfferMintMismatch,

    /// Invalid token_in mint
    #[msg("Invalid token_in mint")]
    InvalidTokenInMint,

    /// Invalid token_out mint
    #[msg("Invalid token_out mint")]
    InvalidTokenOutMint,

    /// Redemption offer account is not owned by this program
    #[msg("Invalid redemption offer owner")]
    InvalidRedemptionOfferOwner,

    /// Redemption offer account data is invalid
    #[msg("Invalid redemption offer data")]
    InvalidRedemptionOfferData,

    /// Redemption fee vault authority PDA is invalid
    #[msg("Invalid redemption fee vault authority")]
    InvalidRedemptionFeeVaultAuthority,

    /// Redemption fee vault authority account is not owned by this program
    #[msg("Invalid redemption fee vault authority owner")]
    InvalidRedemptionFeeVaultAuthorityOwner,

    /// Redemption fee vault authority account data is invalid
    #[msg("Invalid redemption fee vault authority data")]
    InvalidRedemptionFeeVaultAuthorityData,

    /// Invalid redeemer
    #[msg("Redeemer does not match redemption request")]
    InvalidRedeemer,

    /// amount parameter is zero
    #[msg("Invalid amount: must be greater than zero")]
    InvalidAmount,

    /// amount exceeds the remaining unfulfilled balance of the request
    #[msg("Amount exceeds remaining unfulfilled balance")]
    AmountExceedsRemaining,

    /// Arithmetic overflow occurred
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,

    /// Arithmetic underflow occurred
    #[msg("Arithmetic underflow")]
    ArithmeticUnderflow,

    /// The provided fee_destination account does not match the expected fee destination
    #[msg("Invalid fee destination account")]
    InvalidFeeDestination,
}
