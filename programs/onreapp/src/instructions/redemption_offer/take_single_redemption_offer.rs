use crate::constants::seeds;
use crate::instructions::{SingleRedemptionOffer, SingleRedemptionOfferAccount};
use crate::state::State;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[error_code]
pub enum TakeSingleRedemptionOfferErrorCode {
    #[msg("Offer not found")]
    OfferNotFound,
    #[msg("Invalid token in mint")]
    InvalidTokenInMint,
    #[msg("Invalid token out mint")]
    InvalidTokenOutMint,
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
pub struct TakeSingleRedemptionOfferEvent {
    pub offer_id: u64,
    pub token_in_amount: u64,
    pub token_out_amount: u64,
    pub user: Pubkey,
}

/// Account structure for taking a single redemption offer.
///
/// This struct defines the accounts required for a user to take a redemption offer.
/// The user provides token_in and receives token_out based on the offer's price.
#[derive(Accounts)]
#[instruction(offer_id: u64)]
pub struct TakeSingleRedemptionOffer<'info> {
    /// The single redemption offer account containing all offers.
    #[account(mut, seeds = [seeds::SINGLE_REDEMPTION_OFFERS], bump)]
    pub single_redemption_offer_account: AccountLoader<'info, SingleRedemptionOfferAccount>,

    /// Program state to get the boss.
    pub state: Box<Account<'info, State>>,

    /// The boss account that receives token_in payments.
    /// This must match the boss in the state account.
    #[account(
        constraint = boss.key() == state.boss @ TakeSingleRedemptionOfferErrorCode::InvalidBoss
    )]
    /// CHECK: This account is validated through the constraint above
    pub boss: UncheckedAccount<'info>,

    /// The vault authority that controls vault token accounts.
    #[account(seeds = [seeds::VAULT_AUTHORITY], bump)]
    /// CHECK: This is safe as it's a PDA used for signing
    pub vault_authority: UncheckedAccount<'info>,

    /// The token mint for token_in.
    pub token_in_mint: Box<Account<'info, Mint>>,

    /// The token mint for token_out.
    pub token_out_mint: Box<Account<'info, Mint>>,

    /// User's token_in account (source of payment).
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = user
    )]
    pub user_token_in_account: Box<Account<'info, TokenAccount>>,

    /// User's token_out account (destination of tokens).
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = user
    )]
    pub user_token_out_account: Box<Account<'info, TokenAccount>>,

    /// Boss's token_in account (destination of payment).
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = boss
    )]
    pub boss_token_in_account: Box<Account<'info, TokenAccount>>,

    /// Vault's token_out account (source of tokens to give).
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = vault_authority
    )]
    pub vault_token_out_account: Box<Account<'info, TokenAccount>>,

    /// The user taking the offer.
    #[account(mut)]
    pub user: Signer<'info>,

    /// SPL Token program.
    pub token_program: Program<'info, Token>,
}

/// Takes a single redemption offer.
///
/// Allows a user to exchange token_in for token_out based on the offer's price.
/// The price is stored with 9 decimal precision, so 1.000000001 = 1000000001.
/// 
/// Calculation: if user provides 10 token_in and price is 2, user gets 5 token_out.
/// Formula: token_out_amount = (token_in_amount * 10^(token_out_decimals + 9)) / (price * 10^token_in_decimals)
///
/// # Arguments
/// - `ctx`: Context containing the accounts for taking the offer.
/// - `offer_id`: ID of the offer to take.
/// - `token_in_amount`: Amount of token_in to provide.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn take_single_redemption_offer(
    ctx: Context<TakeSingleRedemptionOffer>,
    offer_id: u64,
    token_in_amount: u64,
) -> Result<()> {
    // Find the offer
    let offer = find_offer(&ctx, offer_id)?;
    
    // Validate the offer
    validate_offer(&ctx, &offer)?;
    
    // Calculate token_out_amount
    let token_out_amount = calculate_token_out_amount(
        token_in_amount,
        offer.price,
        ctx.accounts.token_in_mint.decimals,
        ctx.accounts.token_out_mint.decimals,
    )?;
    
    // Execute transfers
    execute_transfers(&ctx, token_in_amount, token_out_amount)?;
    
    msg!(
        "Redemption offer taken - ID: {}, token_in: {}, token_out: {}, user: {}",
        offer_id,
        token_in_amount,
        token_out_amount,
        ctx.accounts.user.key()
    );
    
    // Emit event
    emit!(TakeSingleRedemptionOfferEvent {
        offer_id,
        token_in_amount,
        token_out_amount,
        user: ctx.accounts.user.key(),
    });

    Ok(())
}

/// Finds the offer by ID (pure data retrieval)
fn find_offer(
    ctx: &Context<TakeSingleRedemptionOffer>,
    offer_id: u64,
) -> Result<SingleRedemptionOffer> {
    if offer_id == 0 {
        return Err(error!(TakeSingleRedemptionOfferErrorCode::OfferNotFound));
    }

    let single_redemption_offer_account = &ctx.accounts.single_redemption_offer_account.load()?;

    // Find the offer
    let offer = single_redemption_offer_account
        .offers
        .iter()
        .find(|offer| offer.offer_id == offer_id)
        .ok_or(TakeSingleRedemptionOfferErrorCode::OfferNotFound)?;

    Ok(*offer)
}

/// Validates all offer conditions (business rules)
fn validate_offer(
    ctx: &Context<TakeSingleRedemptionOffer>,
    offer: &SingleRedemptionOffer,
) -> Result<()> {
    // Validate token mints match the offer
    if offer.token_in_mint != ctx.accounts.token_in_mint.key() {
        return Err(error!(TakeSingleRedemptionOfferErrorCode::InvalidTokenInMint));
    }
    if offer.token_out_mint != ctx.accounts.token_out_mint.key() {
        return Err(error!(TakeSingleRedemptionOfferErrorCode::InvalidTokenOutMint));
    }

    // Validate offer timing
    let current_time = Clock::get()?.unix_timestamp as u64;
    if current_time < offer.start_time {
        return Err(error!(TakeSingleRedemptionOfferErrorCode::OfferNotActive));
    }
    if current_time >= offer.end_time {
        return Err(error!(TakeSingleRedemptionOfferErrorCode::OfferExpired));
    }
    
    Ok(())
}

/// Calculates the token_out_amount based on token_in_amount, price, and decimals
fn calculate_token_out_amount(
    token_in_amount: u64,
    price: u64,
    token_in_decimals: u8,
    token_out_decimals: u8,
) -> Result<u64> {
    // Formula: token_out_amount = (token_in_amount * 10^(token_out_decimals + 9)) / (price * 10^token_in_decimals)
    let token_in_amount_u128 = token_in_amount as u128;
    let price_u128 = price as u128;
    
    // Calculate: numerator = token_in_amount * 10^(token_out_decimals + 9)
    let numerator = token_in_amount_u128
        .checked_mul(10_u128.pow((token_out_decimals + 9) as u32))
        .ok_or(TakeSingleRedemptionOfferErrorCode::MathOverflow)?;
    
    // Calculate: denominator = price * 10^token_in_decimals
    let denominator = price_u128
        .checked_mul(10_u128.pow(token_in_decimals as u32))
        .ok_or(TakeSingleRedemptionOfferErrorCode::MathOverflow)?;
    
    Ok((numerator / denominator) as u64)
}

/// Executes both token transfers (user to boss, vault to user)
fn execute_transfers(
    ctx: &Context<TakeSingleRedemptionOffer>,
    token_in_amount: u64,
    token_out_amount: u64,
) -> Result<()> {
    // Transfer token_in from user to boss
    transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.user_token_in_account,
        &ctx.accounts.boss_token_in_account,
        &ctx.accounts.user,
        None,
        token_in_amount,
    )?;
    
    // Transfer token_out from vault to user using vault authority
    let vault_authority_bump = ctx.bumps.vault_authority;
    let vault_authority_seeds = &[
        seeds::VAULT_AUTHORITY,
        &[vault_authority_bump],
    ];
    let signer_seeds = &[vault_authority_seeds.as_slice()];

    transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.vault_token_out_account,
        &ctx.accounts.user_token_out_account,
        &ctx.accounts.vault_authority,
        Some(signer_seeds),
        token_out_amount,
    )?;
    
    Ok(())
}