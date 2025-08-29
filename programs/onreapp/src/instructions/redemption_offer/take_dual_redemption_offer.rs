use crate::constants::seeds;
use crate::instructions::{DualRedemptionOffer, DualRedemptionOfferAccount};
use crate::state::State;
use crate::utils::{calculate_fees, calculate_token_out_amount, transfer_tokens};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[error_code]
pub enum TakeDualRedemptionOfferErrorCode {
    #[msg("Offer not found")]
    OfferNotFound,
    #[msg("Invalid token in mint")]
    InvalidTokenInMint,
    #[msg("Invalid token out mint 1")]
    InvalidTokenOutMint1,
    #[msg("Invalid token out mint 2")]
    InvalidTokenOutMint2,
    #[msg("Offer has expired")]
    OfferExpired,
    #[msg("Offer not yet active")]
    OfferNotActive,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Invalid boss account")]
    InvalidBoss,
}

#[event]
pub struct TakeDualRedemptionOfferEvent {
    pub offer_id: u64,
    pub token_in_amount: u64,
    pub token_out_1_amount: u64,
    pub token_out_2_amount: u64,
    pub fee_amount: u64,
    pub user: Pubkey,
}

/// Account structure for taking a dual redemption offer.
///
/// This struct defines the accounts required for a user to take a dual redemption offer.
/// The user provides token_in and receives token_out_1 and token_out_2 based on the offer's prices and ratio.
#[derive(Accounts)]
#[instruction(offer_id: u64)]
pub struct TakeDualRedemptionOffer<'info> {
    /// The dual redemption offer account containing all offers.
    #[account(mut, seeds = [seeds::DUAL_REDEMPTION_OFFERS], bump)]
    pub dual_redemption_offer_account: AccountLoader<'info, DualRedemptionOfferAccount>,

    /// Program state to get the boss.
    pub state: Box<Account<'info, State>>,

    /// The boss account that receives token_in payments.
    /// This must match the boss in the state account.
    #[account(
        constraint = boss.key() == state.boss @ TakeDualRedemptionOfferErrorCode::InvalidBoss
    )]
    /// CHECK: This account is validated through the constraint above
    pub boss: UncheckedAccount<'info>,

    /// The vault authority that controls vault token accounts.
    #[account(seeds = [seeds::VAULT_AUTHORITY], bump)]
    /// CHECK: This is safe as it's a PDA used for signing
    pub vault_authority: UncheckedAccount<'info>,

    /// The token mint for token_in.
    pub token_in_mint: Box<Account<'info, Mint>>,

    /// The token mint for token_out_1.
    pub token_out_mint_1: Box<Account<'info, Mint>>,

    /// The token mint for token_out_2.
    pub token_out_mint_2: Box<Account<'info, Mint>>,

    /// User's token_in account (source of payment).
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = user
    )]
    pub user_token_in_account: Box<Account<'info, TokenAccount>>,

    /// User's token_out_1 account (destination of token_out_1).
    #[account(
        mut,
        associated_token::mint = token_out_mint_1,
        associated_token::authority = user
    )]
    pub user_token_out_1_account: Box<Account<'info, TokenAccount>>,

    /// User's token_out_2 account (destination of token_out_2).
    #[account(
        mut,
        associated_token::mint = token_out_mint_2,
        associated_token::authority = user
    )]
    pub user_token_out_2_account: Box<Account<'info, TokenAccount>>,

    /// Boss's token_in account (destination of payment).
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = boss
    )]
    pub boss_token_in_account: Box<Account<'info, TokenAccount>>,

    /// Vault's token_out_1 account (source of token_out_1 to give).
    #[account(
        mut,
        associated_token::mint = token_out_mint_1,
        associated_token::authority = vault_authority
    )]
    pub vault_token_out_1_account: Box<Account<'info, TokenAccount>>,

    /// Vault's token_out_2 account (source of token_out_2 to give).
    #[account(
        mut,
        associated_token::mint = token_out_mint_2,
        associated_token::authority = vault_authority
    )]
    pub vault_token_out_2_account: Box<Account<'info, TokenAccount>>,

    /// The user taking the offer.
    #[account(mut)]
    pub user: Signer<'info>,

    /// SPL Token program.
    pub token_program: Program<'info, Token>,
}

/// Takes a dual redemption offer.
///
/// Allows a user to exchange token_in for token_out_1 and token_out_2 based on the offer's prices and ratio.
/// The prices represent how much token_in is needed to get 1 token_out (with 9 decimal precision).
/// The ratio determines how much of the total token_in amount goes to each output token.
///
/// Example: if price_1 is 2000000000 (2.0), price_2 is 1000000000 (1.0), and ratio is 8000 (80%):
/// - 80% of token_in is used to get token_out_1 at price_1
/// - 20% of token_in is used to get token_out_2 at price_2
///
/// # Arguments
/// - `ctx`: Context containing the accounts for taking the offer.
/// - `offer_id`: ID of the offer to take.
/// - `token_in_amount`: Amount of token_in to provide.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn take_dual_redemption_offer(
    ctx: Context<TakeDualRedemptionOffer>,
    offer_id: u64,
    token_in_amount: u64,
) -> Result<()> {
    // Find the offer
    let offer = find_offer(&ctx, offer_id)?;

    // Validate the offer
    validate_offer(&ctx, &offer)?;

    // Calculate fee amount in token_in tokens
    let fee_amounts = calculate_fees(token_in_amount, offer.fee_basis_points)?;

    // Calculate token_out amounts based on remaining amount after fee
    let (token_out_1_amount, token_out_2_amount) = calculate_token_out_amounts(
        fee_amounts.remaining_token_in_amount,
        offer.price_1,
        offer.price_2,
        offer.ratio_basis_points,
        ctx.accounts.token_in_mint.decimals,
        ctx.accounts.token_out_mint_1.decimals,
        ctx.accounts.token_out_mint_2.decimals,
    )?;

    // Execute transfers
    execute_transfers(
        &ctx,
        token_in_amount,
        token_out_1_amount,
        token_out_2_amount,
    )?;

    msg!(
        "Dual redemption offer taken - ID: {}, token_in: {}, fee: {}, token_out_1: {}, token_out_2: {}, user: {}",
        offer_id,
        token_in_amount,
        fee_amounts.fee_amount,
        token_out_1_amount,
        token_out_2_amount,
        ctx.accounts.user.key()
    );

    // Emit event
    emit!(TakeDualRedemptionOfferEvent {
        offer_id,
        token_in_amount,
        token_out_1_amount,
        token_out_2_amount,
        fee_amount: fee_amounts.fee_amount,
        user: ctx.accounts.user.key(),
    });

    Ok(())
}

/// Finds the offer by ID (pure data retrieval)
fn find_offer(
    ctx: &Context<TakeDualRedemptionOffer>,
    offer_id: u64,
) -> Result<DualRedemptionOffer> {
    require!(
        offer_id != 0,
        TakeDualRedemptionOfferErrorCode::OfferNotFound
    );

    let dual_redemption_offer_account = &ctx.accounts.dual_redemption_offer_account.load()?;

    // Find the offer
    let offer = dual_redemption_offer_account
        .offers
        .iter()
        .find(|offer| offer.offer_id == offer_id)
        .ok_or(TakeDualRedemptionOfferErrorCode::OfferNotFound)?;

    Ok(*offer)
}

/// Validates all offer conditions (business rules)
fn validate_offer(
    ctx: &Context<TakeDualRedemptionOffer>,
    offer: &DualRedemptionOffer,
) -> Result<()> {
    // Validate token mints match the offer
    require!(
        offer.token_in_mint == ctx.accounts.token_in_mint.key(),
        TakeDualRedemptionOfferErrorCode::InvalidTokenInMint
    );
    require!(
        offer.token_out_mint_1 == ctx.accounts.token_out_mint_1.key(),
        TakeDualRedemptionOfferErrorCode::InvalidTokenOutMint1
    );
    require!(
        offer.token_out_mint_2 == ctx.accounts.token_out_mint_2.key(),
        TakeDualRedemptionOfferErrorCode::InvalidTokenOutMint2
    );

    // Validate offer timing
    let current_time = Clock::get()?.unix_timestamp as u64;
    require!(
        current_time >= offer.start_time,
        TakeDualRedemptionOfferErrorCode::OfferNotActive
    );
    require!(
        current_time < offer.end_time,
        TakeDualRedemptionOfferErrorCode::OfferExpired
    );

    Ok(())
}

/// Calculates the token_out amounts based on token_in_amount, prices, ratio, and decimals
///
/// The ratio determines how the token_in amount is split between the two output tokens:
/// - ratio_basis_points out of 10000 goes to token_out_1
/// - (10000 - ratio_basis_points) out of 10000 goes to token_out_2
///
/// Then each portion is converted using its respective price.
fn calculate_token_out_amounts(
    token_in_amount: u64,
    price_1: u64,
    price_2: u64,
    ratio_basis_points: u64,
    token_in_decimals: u8,
    token_out_1_decimals: u8,
    token_out_2_decimals: u8,
) -> Result<(u64, u64)> {
    let token_in_amount_u128 = token_in_amount as u128;
    let ratio_1_u128 = ratio_basis_points as u128;
    let ratio_2_u128 = (10000 - ratio_basis_points) as u128;

    // Split token_in amount based on ratio
    // token_in_1 = token_in_amount * ratio_basis_points / 10000
    let token_in_1_amount = token_in_amount_u128
        .checked_mul(ratio_1_u128)
        .ok_or(TakeDualRedemptionOfferErrorCode::MathOverflow)?
        .checked_div(10000)
        .ok_or(TakeDualRedemptionOfferErrorCode::MathOverflow)? as u64;

    // token_in_2 = token_in_amount * (10000 - ratio_basis_points) / 10000
    let token_in_2_amount = token_in_amount_u128
        .checked_mul(ratio_2_u128)
        .ok_or(TakeDualRedemptionOfferErrorCode::MathOverflow)?
        .checked_div(10000)
        .ok_or(TakeDualRedemptionOfferErrorCode::MathOverflow)? as u64;

    // Calculate token_out_1_amount using price_1
    let token_out_1_amount = calculate_token_out_amount(
        token_in_1_amount,
        price_1,
        token_in_decimals,
        token_out_1_decimals,
    )?;

    // Calculate token_out_2_amount using price_2
    let token_out_2_amount = calculate_token_out_amount(
        token_in_2_amount,
        price_2,
        token_in_decimals,
        token_out_2_decimals,
    )?;

    Ok((token_out_1_amount, token_out_2_amount))
}

/// Executes all token transfers (single transfer for total amount including fee)
fn execute_transfers(
    ctx: &Context<TakeDualRedemptionOffer>,
    total_token_in_amount: u64,
    token_out_1_amount: u64,
    token_out_2_amount: u64,
) -> Result<()> {
    // Transfer total token_in from user to boss (includes fee)
    transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.user_token_in_account,
        &ctx.accounts.boss_token_in_account,
        &ctx.accounts.user,
        None,
        total_token_in_amount,
    )?;

    // Prepare vault authority seeds for signing transfers from vault
    let vault_authority_bump = ctx.bumps.vault_authority;
    let vault_authority_seeds = &[seeds::VAULT_AUTHORITY, &[vault_authority_bump]];
    let signer_seeds = &[vault_authority_seeds.as_slice()];

    // Transfer token_out_1 from vault to user using vault authority
    transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.vault_token_out_1_account,
        &ctx.accounts.user_token_out_1_account,
        &ctx.accounts.vault_authority,
        Some(signer_seeds),
        token_out_1_amount,
    )?;

    // Transfer token_out_2 from vault to user using vault authority
    transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.vault_token_out_2_account,
        &ctx.accounts.user_token_out_2_account,
        &ctx.accounts.vault_authority,
        Some(signer_seeds),
        token_out_2_amount,
    )?;

    Ok(())
}
