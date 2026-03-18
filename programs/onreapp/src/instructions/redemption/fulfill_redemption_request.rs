use crate::constants::seeds;
use crate::instructions::buffer::{
    validate_buffer_onyc_vault_accounts, BufferAccrualAccounts,
    manage_buffer::{accrue_buffer, set_buffer_baseline_after_supply_change},
    BufferErrorCode,
};
use crate::instructions::buffer::accounts::{
    __client_accounts_buffer_accrual_accounts, __cpi_client_accounts_buffer_accrual_accounts,
    BufferAccrualAccountsBumps,
};
use crate::instructions::redemption::{
    execute_redemption_operations, process_redemption_core, ExecuteRedemptionOpsParams,
    RedemptionOffer, RedemptionRequest,
};
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::program_controls_mint;
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::{
    associated_token::AssociatedToken,
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

    /// The redemption offer account
    #[account(
        mut,
        seeds = [
            seeds::REDEMPTION_OFFER,
            redemption_offer.token_in_mint.as_ref(),
            redemption_offer.token_out_mint.as_ref()
        ],
        bump = redemption_offer.bump,
        constraint = redemption_offer.offer == offer.key()
            @ FulfillRedemptionRequestErrorCode::OfferMismatch
    )]
    pub redemption_offer: Box<Account<'info, RedemptionOffer>>,

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
    #[account(
        mut,
        constraint = token_in_mint.key() == redemption_offer.token_in_mint
            @ FulfillRedemptionRequestErrorCode::InvalidTokenInMint
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for input token operations
    pub token_in_program: Interface<'info, TokenInterface>,

    /// Output token mint (typically stablecoin like USDC)
    ///
    /// Must be mutable to allow minting operations when program has mint authority.
    #[account(
        mut,
        constraint = token_out_mint.key() == redemption_offer.token_out_mint
            @ FulfillRedemptionRequestErrorCode::InvalidTokenOutMint
    )]
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

    /// Boss's input token account for receiving tokens when program lacks mint authority
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
}

#[derive(Accounts)]
pub struct FulfillRedemptionRequestExtended<'info> {
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

    #[account(
        mut,
        seeds = [
            seeds::REDEMPTION_OFFER,
            redemption_offer.token_in_mint.as_ref(),
            redemption_offer.token_out_mint.as_ref()
        ],
        bump = redemption_offer.bump,
        constraint = redemption_offer.offer == offer.key()
            @ FulfillRedemptionRequestErrorCode::OfferMismatch
    )]
    pub redemption_offer: Box<Account<'info, RedemptionOffer>>,

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

    #[account(
        mut,
        constraint = token_in_mint.key() == redemption_offer.token_in_mint
            @ FulfillRedemptionRequestErrorCode::InvalidTokenInMint
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_in_program: Interface<'info, TokenInterface>,

    #[account(
        mut,
        constraint = token_out_mint.key() == redemption_offer.token_out_mint
            @ FulfillRedemptionRequestErrorCode::InvalidTokenOutMint
    )]
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
    execute_fulfill_redemption_request(
        ExecuteFulfillRedemptionRequestParams {
            program_id: ctx.program_id,
            state: &ctx.accounts.state,
            offer: &ctx.accounts.offer,
            redemption_offer: &mut ctx.accounts.redemption_offer,
            redemption_request: &mut ctx.accounts.redemption_request,
            vault_token_in_account: &ctx.accounts.vault_token_in_account,
            vault_token_out_account: &ctx.accounts.vault_token_out_account,
            token_in_mint: &ctx.accounts.token_in_mint,
            token_in_program: &ctx.accounts.token_in_program,
            token_out_mint: &ctx.accounts.token_out_mint,
            token_out_program: &ctx.accounts.token_out_program,
            user_token_out_account: &ctx.accounts.user_token_out_account,
            boss_token_in_account: &ctx.accounts.boss_token_in_account,
            mint_authority: &ctx.accounts.mint_authority,
            redemption_vault_authority: &ctx.accounts.redemption_vault_authority,
            redemption_vault_authority_bump: ctx.bumps.redemption_vault_authority,
            mint_authority_bump: ctx.bumps.mint_authority,
            redeemer: &ctx.accounts.redeemer,
            redemption_admin: &ctx.accounts.redemption_admin,
            buffer_accounts: None,
        },
        amount,
    )
}

pub fn fulfill_redemption_request_extended(
    ctx: Context<FulfillRedemptionRequestExtended>,
    amount: u64,
) -> Result<()> {
    execute_fulfill_redemption_request(
        ExecuteFulfillRedemptionRequestParams {
            program_id: ctx.program_id,
            state: &ctx.accounts.state,
            offer: &ctx.accounts.offer,
            redemption_offer: &mut ctx.accounts.redemption_offer,
            redemption_request: &mut ctx.accounts.redemption_request,
            vault_token_in_account: &ctx.accounts.vault_token_in_account,
            vault_token_out_account: &ctx.accounts.vault_token_out_account,
            token_in_mint: &ctx.accounts.token_in_mint,
            token_in_program: &ctx.accounts.token_in_program,
            token_out_mint: &ctx.accounts.token_out_mint,
            token_out_program: &ctx.accounts.token_out_program,
            user_token_out_account: &ctx.accounts.user_token_out_account,
            boss_token_in_account: &ctx.accounts.boss_token_in_account,
            mint_authority: &ctx.accounts.mint_authority,
            redemption_vault_authority: &ctx.accounts.redemption_vault_authority,
            redemption_vault_authority_bump: ctx.bumps.redemption_vault_authority,
            mint_authority_bump: ctx.bumps.mint_authority,
            redeemer: &ctx.accounts.redeemer,
            redemption_admin: &ctx.accounts.redemption_admin,
            buffer_accounts: Some(&ctx.accounts.buffer_accounts),
        },
        amount,
    )
}

struct ExecuteFulfillRedemptionRequestParams<'a, 'info> {
    program_id: &'a Pubkey,
    state: &'a Account<'info, State>,
    offer: &'a AccountLoader<'info, Offer>,
    redemption_offer: &'a mut Account<'info, RedemptionOffer>,
    redemption_request: &'a mut Account<'info, RedemptionRequest>,
    vault_token_in_account: &'a InterfaceAccount<'info, TokenAccount>,
    vault_token_out_account: &'a InterfaceAccount<'info, TokenAccount>,
    token_in_mint: &'a InterfaceAccount<'info, Mint>,
    token_in_program: &'a Interface<'info, TokenInterface>,
    token_out_mint: &'a InterfaceAccount<'info, Mint>,
    token_out_program: &'a Interface<'info, TokenInterface>,
    user_token_out_account: &'a InterfaceAccount<'info, TokenAccount>,
    boss_token_in_account: &'a InterfaceAccount<'info, TokenAccount>,
    mint_authority: &'a UncheckedAccount<'info>,
    redemption_vault_authority: &'a UncheckedAccount<'info>,
    redemption_vault_authority_bump: u8,
    mint_authority_bump: u8,
    redeemer: &'a UncheckedAccount<'info>,
    redemption_admin: &'a Signer<'info>,
    buffer_accounts: Option<&'a BufferAccrualAccounts<'info>>,
}

fn execute_fulfill_redemption_request<'info>(
    mut params: ExecuteFulfillRedemptionRequestParams<'_, 'info>,
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

    if let Some(buffer_accounts) = params.buffer_accounts.filter(|_| {
        params.token_in_mint.key() == params.state.onyc_mint
            && program_controls_mint(params.token_in_mint, &params.mint_authority.to_account_info())
    }) {
        let mut buffer_state = buffer_accounts.load_buffer_state()?;
        validate_buffer_onyc_vault_accounts(
            params.program_id,
            &buffer_state,
            &buffer_accounts.buffer_vault_onyc_account_info(),
            &buffer_accounts.management_fee_vault_onyc_account_info(),
            &buffer_accounts.performance_fee_vault_onyc_account_info(),
            params.token_in_mint,
            params.token_in_program,
        )?;

        let now = Clock::get()?.unix_timestamp;
        let accrual = accrue_buffer(
            params.state,
            &mut buffer_state,
            &offer,
            params.token_in_mint,
            buffer_accounts.buffer_vault_onyc_account_info(),
            buffer_accounts.management_fee_vault_onyc_account_info(),
            buffer_accounts.performance_fee_vault_onyc_account_info(),
            params.mint_authority.to_account_info(),
            params.mint_authority_bump,
            params.token_in_program,
            now,
        )?;

        execute_redemption_operations(ExecuteRedemptionOpsParams {
            token_in_program: params.token_in_program,
            token_out_program: params.token_out_program,
            token_in_mint: params.token_in_mint,
            token_in_net_amount,
            token_in_fee_amount,
            vault_token_in_account: params.vault_token_in_account,
            boss_token_in_account: params.boss_token_in_account,
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

        let post_burn_supply = accrual
            .post_accrual_supply
            .checked_sub(token_in_net_amount)
            .ok_or(BufferErrorCode::MathOverflow)?;
        set_buffer_baseline_after_supply_change(&mut buffer_state, post_burn_supply, now);
        buffer_accounts.store_buffer_state(&buffer_state)?;
    } else {
        drop(offer);

        // Execute token operations (burn/transfer token_in_net, mint/transfer token_out)
        // Fee transfer is handled inside execute_redemption_operations
        execute_redemption_operations(ExecuteRedemptionOpsParams {
            token_in_program: params.token_in_program,
            token_out_program: params.token_out_program,
            token_in_mint: params.token_in_mint,
            token_in_net_amount,
            token_in_fee_amount,
            vault_token_in_account: params.vault_token_in_account,
            boss_token_in_account: params.boss_token_in_account,
            redemption_vault_authority: &params.redemption_vault_authority.to_account_info(),
            redemption_vault_authority_bump: params.redemption_vault_authority_bump,
            token_out_mint: params.token_out_mint,
            token_out_amount,
            vault_token_out_account: params.vault_token_out_account,
            user_token_out_account: params.user_token_out_account,
            mint_authority_pda: &params.mint_authority.to_account_info(),
            mint_authority_bump: params.mint_authority_bump,
            token_out_max_supply: 0, // No max supply cap for redemptions
        })?;
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
        redemption_offer_pda: params.redemption_offer.key(),
        redeemer: params.redeemer.key(),
        token_in_net_amount,
        token_in_fee_amount,
        token_out_amount,
        current_price: price,
        fulfilled_amount: amount,
        total_fulfilled_amount: new_fulfilled_amount,
        is_fully_fulfilled,
    });

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
}
