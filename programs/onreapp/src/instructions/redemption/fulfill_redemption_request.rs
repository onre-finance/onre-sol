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
    get_associated_token_account, get_or_create_associated_token_account, load_or_init_pda_account,
    load_pda_account, program_controls_mint, store_pda_account, validate_associated_token_address,
    EnsureAtaParams, PdaAccountInit,
};
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

#[derive(Accounts)]
pub struct FulfillRedemptionRequest<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ crate::OnreError::InvalidBoss,
        constraint = !state.is_killed @ crate::OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// CHECK: Account validation is enforced through state account constraint
    pub boss: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against the loaded redemption offer.
    pub offer: UncheckedAccount<'info>,

    /// CHECK: Validated and stored manually in instruction logic.
    #[account(mut)]
    pub redemption_offer: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic for PDA, ownership, and offer linkage.
    #[account(mut)]
    pub redemption_request: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY],
        bump
    )]
    pub redemption_vault_authority: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against the expected redemption vault ATA.
    #[account(mut)]
    pub vault_token_in_account: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against the expected redemption vault ATA.
    #[account(mut)]
    pub vault_token_out_account: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic via InterfaceAccount deserialization and redemption offer mint checks.
    #[account(mut)]
    pub token_in_mint: UncheckedAccount<'info>,

    pub token_in_program: Interface<'info, TokenInterface>,

    /// CHECK: Validated in instruction logic via InterfaceAccount deserialization and redemption offer mint checks.
    #[account(mut)]
    pub token_out_mint: UncheckedAccount<'info>,

    pub token_out_program: Interface<'info, TokenInterface>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub user_token_out_account: UncheckedAccount<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub boss_token_in_account: UncheckedAccount<'info>,

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
    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub fee_destination_token_in_account: UncheckedAccount<'info>,

    /// CHECK: PDA derivation is validated through seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against redemption_request.redeemer.
    pub redeemer: UncheckedAccount<'info>,

    #[account(mut)]
    pub redemption_admin: Signer<'info>,

    pub buffer_accounts: BufferAccrualAccounts<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub offer_vault_authority: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against the canonical ATA derivation.
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
pub fn fulfill_redemption_request<'info>(
    ctx: Context<'info, FulfillRedemptionRequest<'info>>,
    amount: u64,
) -> Result<()> {
    msg!("fulfill: loading offer/request/mints");
    let offer = AccountLoader::<Offer>::try_from(&ctx.accounts.offer)?;
    let mut redemption_request =
        Account::<RedemptionRequest>::try_from(&ctx.accounts.redemption_request)?;
    let mut token_in_mint = InterfaceAccount::<Mint>::try_from(&ctx.accounts.token_in_mint)?;
    let token_out_mint = InterfaceAccount::<Mint>::try_from(&ctx.accounts.token_out_mint)?;
    let (expected_redemption_request, _) = Pubkey::find_program_address(
        &[
            seeds::REDEMPTION_REQUEST,
            redemption_request.offer.as_ref(),
            redemption_request.request_id.to_le_bytes().as_ref(),
        ],
        ctx.program_id,
    );
    require_keys_eq!(
        ctx.accounts.redemption_request.key(),
        expected_redemption_request,
        crate::OnreError::OfferMismatch
    );
    require_keys_eq!(
        redemption_request.offer,
        ctx.accounts.redemption_offer.key(),
        crate::OnreError::OfferMismatch
    );
    msg!("fulfill: validating redeemer/admin");
    require_keys_eq!(
        ctx.accounts.redeemer.key(),
        redemption_request.redeemer,
        crate::OnreError::InvalidRedeemer
    );
    require_keys_eq!(
        ctx.accounts.redemption_admin.key(),
        ctx.accounts.state.redemption_admin,
        crate::OnreError::Unauthorized
    );
    msg!("fulfill: validating offer vault onyc ata");
    validate_associated_token_address(
        &ctx.accounts.offer_vault_onyc_account,
        &ctx.accounts.offer_vault_authority.key(),
        &token_in_mint.key(),
        &ctx.accounts.token_in_program.key(),
        crate::OnreError::InvalidOfferVaultOnycAccount,
    )?;
    msg!("fulfill: validating redemption vault token in ata");
    let vault_token_in_account = get_associated_token_account(
        &ctx.accounts.vault_token_in_account,
        &ctx.accounts.redemption_vault_authority.key(),
        &token_in_mint.key(),
        &ctx.accounts.token_in_program.key(),
        crate::OnreError::InvalidVaultTokenInAccount,
    )?;
    msg!("fulfill: validating redemption vault token out ata");
    let vault_token_out_account = get_associated_token_account(
        &ctx.accounts.vault_token_out_account,
        &ctx.accounts.redemption_vault_authority.key(),
        &token_out_mint.key(),
        &ctx.accounts.token_out_program.key(),
        crate::OnreError::InvalidVaultTokenOutAccount,
    )?;
    msg!("fulfill: ensuring user token out ata");
    let user_token_out_account = get_or_create_associated_token_account(EnsureAtaParams {
        ata_account: &ctx.accounts.user_token_out_account,
        payer: ctx.accounts.redemption_admin.to_account_info(),
        authority_account: ctx.accounts.redeemer.to_account_info(),
        mint_account: ctx.accounts.token_out_mint.to_account_info(),
        token_program: ctx.accounts.token_out_program.to_account_info(),
        associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        authority: ctx.accounts.redeemer.key(),
        mint: token_out_mint.key(),
        token_program_id: ctx.accounts.token_out_program.key(),
        invalid_account_error: crate::OnreError::InvalidUserTokenOutAccount,
    })?;
    msg!("fulfill: ensuring boss token in ata");
    let boss_token_in_account = get_or_create_associated_token_account(EnsureAtaParams {
        ata_account: &ctx.accounts.boss_token_in_account,
        payer: ctx.accounts.redemption_admin.to_account_info(),
        authority_account: ctx.accounts.boss.to_account_info(),
        mint_account: ctx.accounts.token_in_mint.to_account_info(),
        token_program: ctx.accounts.token_in_program.to_account_info(),
        associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        authority: ctx.accounts.boss.key(),
        mint: token_in_mint.key(),
        token_program_id: ctx.accounts.token_in_program.key(),
        invalid_account_error: crate::OnreError::InvalidBossTokenInAccount,
    })?;
    msg!("fulfill: ensuring fee destination token in ata");
    let fee_destination_token_in_account =
        get_or_create_associated_token_account(EnsureAtaParams {
            ata_account: &ctx.accounts.fee_destination_token_in_account,
            payer: ctx.accounts.redemption_admin.to_account_info(),
            authority_account: ctx.accounts.fee_destination.to_account_info(),
            mint_account: ctx.accounts.token_in_mint.to_account_info(),
            token_program: ctx.accounts.token_in_program.to_account_info(),
            associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            authority: ctx.accounts.fee_destination.key(),
            mint: token_in_mint.key(),
            token_program_id: ctx.accounts.token_in_program.key(),
            invalid_account_error: crate::OnreError::InvalidFeeDestinationTokenInAccount,
        })?;
    let mut redemption_offer = load_redemption_offer(
        ctx.program_id,
        &offer,
        &ctx.accounts.redemption_offer,
        &token_in_mint,
        &token_out_mint,
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
            offer: &offer,
            redemption_offer_account: &ctx.accounts.redemption_offer,
            redemption_offer: &mut redemption_offer,
            redemption_request: &mut redemption_request,
            redemption_fee_vault_authority_account: &ctx.accounts.redemption_fee_vault_authority,
            redemption_fee_vault_authority: &mut redemption_fee_vault_authority,
            vault_token_in_account: &vault_token_in_account,
            vault_token_out_account: &vault_token_out_account,
            token_in_mint: &mut token_in_mint,
            token_in_program: &ctx.accounts.token_in_program,
            token_out_mint: &token_out_mint,
            token_out_program: &ctx.accounts.token_out_program,
            user_token_out_account: &user_token_out_account,
            boss_token_in_account: &boss_token_in_account,
            fee_destination: &ctx.accounts.fee_destination,
            fee_destination_token_in_account: &fee_destination_token_in_account,
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
        error!(crate::OnreError::InvalidRedemptionFeeVaultAuthorityOwner)
    }

    fn invalid_data_error() -> Error {
        error!(crate::OnreError::InvalidRedemptionFeeVaultAuthorityData)
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
        crate::OnreError::InvalidRedemptionOfferOwner.into(),
        crate::OnreError::InvalidRedemptionOfferData.into(),
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
        crate::OnreError::OfferMismatch
    );
    require_keys_eq!(
        redemption_offer.offer,
        offer.key(),
        crate::OnreError::OfferMismatch
    );
    require_keys_eq!(
        token_in_mint.key(),
        redemption_offer.token_in_mint,
        crate::OnreError::InvalidTokenInMint
    );
    require_keys_eq!(
        token_out_mint.key(),
        redemption_offer.token_out_mint,
        crate::OnreError::InvalidTokenOutMint
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
        crate::OnreError::InvalidRedemptionFeeVaultAuthority
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
    require!(amount > 0, crate::OnreError::InvalidAmount);

    let redemption_request = &params.redemption_request;
    let remaining = redemption_request
        .amount
        .checked_sub(redemption_request.fulfilled_amount)
        .ok_or(crate::OnreError::ArithmeticUnderflow)?;

    require!(
        amount <= remaining,
        crate::OnreError::AmountExceedsRemaining
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
        crate::OnreError::InvalidFeeDestination
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
    let buffer_is_initialized = if let Some(accounts) = params.buffer_accounts {
        accounts.check_is_initialized(params.program_id)?
    } else {
        false
    };
    let accrual = if let Some(buffer_accounts) = params
        .buffer_accounts
        .filter(|_| should_refresh_market_stats && buffer_is_initialized)
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
            .ok_or(crate::OnreError::MathOverflow)?;
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
        .ok_or(crate::OnreError::ArithmeticOverflow)?;
    params.redemption_request.fulfilled_amount = new_fulfilled_amount;

    let is_fully_fulfilled = new_fulfilled_amount == params.redemption_request.amount;

    // Update offer-level counters
    let redemption_offer = &mut params.redemption_offer;
    redemption_offer.executed_redemptions = redemption_offer
        .executed_redemptions
        .checked_add(amount as u128)
        .ok_or(crate::OnreError::ArithmeticOverflow)?;

    redemption_offer.requested_redemptions = redemption_offer
        .requested_redemptions
        .checked_sub(amount as u128)
        .ok_or(crate::OnreError::ArithmeticUnderflow)?;

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
    } else {
        params.redemption_request.exit(params.program_id)?;
    }

    Ok(())
}
