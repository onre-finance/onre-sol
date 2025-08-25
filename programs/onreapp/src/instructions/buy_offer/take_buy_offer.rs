use crate::constants::seeds;
use crate::instructions::{BuyOffer, BuyOfferAccount, BuyOfferVector};
use crate::state::State;
use anchor_lang::{instruction, Accounts};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::utils::{calculate_token_out_amount, transfer_tokens, u64_to_dec9};

/// Error codes specific to the take_buy_offer instruction
#[error_code]
pub enum TakeBuyOfferErrorCode {
    #[msg("Offer not found")]   
    OfferNotFound,
    #[msg("Invalid boss account")]
    InvalidBoss,
    #[msg("No active vector")]
    NoActiveVector,
    #[msg("Overflow error")]
    OverflowError
}

/// Event emitted when a buy offer is successfully taken
#[event]
pub struct TakeBuyOfferEvent {
    /// The ID of the buy offer that was taken
    pub offer_id: u64,
    /// Amount of token_in paid by the user
    pub token_in_amount: u64,
    /// Amount of token_out received by the user
    pub token_out_amount: u64,
    /// Public key of the user who took the offer
    pub user: Pubkey,
}

/// Accounts required for taking a buy offer
/// 
/// This struct defines all the accounts needed to execute a take_buy_offer instruction,
/// including validation constraints to ensure security and proper authorization.
#[derive(Accounts)]
#[instruction(offer_id: u64)]
pub struct TakeBuyOffer<'info> {
    /// The buy offer account containing all active buy offers
    #[account(mut, seeds = [seeds::BUY_OFFERS], bump)]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

    /// Program state account containing the boss public key
    pub state: Box<Account<'info, State>>,

    /// The boss account that receives token_in payments
    /// Must match the boss stored in the program state
    #[account(
        constraint = boss.key() == state.boss @ TakeBuyOfferErrorCode::InvalidBoss
    )]
    /// CHECK: This account is validated through the constraint above
    pub boss: UncheckedAccount<'info>,

    /// The vault authority PDA that controls vault token accounts
    #[account(seeds = [seeds::VAULT_AUTHORITY], bump)]
    /// CHECK: This is safe as it's a PDA used for signing
    pub vault_authority: UncheckedAccount<'info>,

    /// The mint account for the input token (what user pays)
    pub token_in_mint: Box<Account<'info, Mint>>,

    /// The mint account for the output token (what user receives)
    pub token_out_mint: Box<Account<'info, Mint>>,

    /// User's token_in account (source of payment)
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = user
    )]
    pub user_token_in_account: Box<Account<'info, TokenAccount>>,

    /// User's token_out account (destination of received tokens)
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = user
    )]
    pub user_token_out_account: Box<Account<'info, TokenAccount>>,

    /// Boss's token_in account (destination of user's payment)
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = boss
    )]
    pub boss_token_in_account: Box<Account<'info, TokenAccount>>,

    /// Vault's token_out account (source of tokens to distribute to user)
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = vault_authority
    )]
    pub vault_token_out_account: Box<Account<'info, TokenAccount>>,

    /// The user taking the offer (must sign the transaction)
    #[account(mut)]
    pub user: Signer<'info>,

    /// SPL Token program for token transfers
    pub token_program: Program<'info, Token>,
}

/// Main instruction handler for taking a buy offer
/// 
/// This function allows users to accept a buy offer by paying the current market price
/// in token_in to receive token_out from the vault. The price is calculated using a
/// discrete interval pricing model with linear yield growth.
/// 
/// # Arguments
/// 
/// * `ctx` - The instruction context containing all required accounts
/// * `offer_id` - The unique ID of the buy offer to take
/// * `token_in_amount` - The amount of token_in the user is willing to pay
/// 
/// # Process Flow
/// 
/// 1. Load and validate the buy offer exists
/// 2. Find the currently active pricing vector
/// 3. Calculate current price based on time elapsed and yield parameters
/// 4. Calculate how many token_out to give for the provided token_in_amount
/// 5. Execute atomic transfers: user → boss (token_in), vault → user (token_out)
/// 6. Emit event with transaction details
/// 
/// # Returns
/// 
/// * `Ok(())` - If the offer was successfully taken
/// * `Err(_)` - If validation fails or transfers cannot be completed
/// 
/// # Errors
/// 
/// * `OfferNotFound` - The specified offer_id doesn't exist
/// * `NoActiveVector` - No pricing vector is currently active  
/// * `OverflowError` - Mathematical overflow in price calculations
/// * Token transfer errors if insufficient balances or invalid accounts
pub fn take_buy_offer(
    ctx: Context<TakeBuyOffer>,
    offer_id: u64,
    token_in_amount: u64,
) -> Result<()> {
    let offer_account = ctx.accounts.buy_offer_account.load_mut()?;
    
    // Find the offer
    let offer = find_offer(&offer_account, offer_id)?;
    
    let active_vector = find_active_vector(&offer)?;

    // Price with 9 decimals !!
    let current_price = calculate_current_price(
        active_vector.price_yield,
        active_vector.start_price,
        active_vector.start_time,
        active_vector.price_fix_duration,
        ctx.accounts.token_in_mint.decimals,
        ctx.accounts.token_out_mint.decimals,
    )?;
    
    let token_out_amount = calculate_token_out_amount(
        token_in_amount,
        current_price,
        ctx.accounts.token_in_mint.decimals,
        ctx.accounts.token_out_mint.decimals,
    )?;
    
    execute_transfers(&ctx, token_in_amount, token_out_amount)?;
    
    msg!(
        "Buy offer taken - ID: {}, token_in: {}, token_out: {}, user: {}, price: {}",
        offer_id, 
        token_in_amount, 
        token_out_amount, 
        ctx.accounts.user.key,
        u64_to_dec9(current_price)
    );
    
    emit!(TakeBuyOfferEvent {
        offer_id,
        token_in_amount,
        token_out_amount,
        user: ctx.accounts.user.key(),
    });
    
    Ok(())
}

/// Linear (uncompounded) price with "price-fix" windows that snap to the END of the current window.
/// - price_yield: yearly yield scaled by 1_000_000 (e.g., 10.12% => 101_200)
/// - start_price: starting price
/// - start_time: epoch seconds when the price starts evolving
/// - price_fix_duration: duration (seconds) of each price-fix window; price is constant within a window
///
/// Formula:
///   k = current interval
///   t = current time
///
///   k = floor((t - start_time) / D)
///   P(t) = P0 * (1 + y * ((k + 1)*D) / S)
/// where S = 365*24*3600, and y is yearly yield as a decimal.
/// We compute this in fixed-point to avoid precision loss.
///
fn calculate_current_price(price_yield: u64, start_price: u64, start_time: u64, price_fix_duration: u64, token_in_decimals: u8, token_out_decimals: u8,
) -> Result<u64> {
    const SCALE: u128 = 1_000_000;          // because price_yield is scaled by 1_000_000
    const S: u64 = 365 * 24 * 3600;         // seconds per year

    let current_time = Clock::get()?.unix_timestamp as u64;

    require!(start_time <= current_time, TakeBuyOfferErrorCode::NoActiveVector);

    let elapsed_since_start = current_time.saturating_sub(start_time);

    // Calculate which price interval we're in (discrete intervals)
    let k = elapsed_since_start / price_fix_duration;

    // elapsed_effective = (k + 1) * D  (end-of-current-interval snap)
    let elapsed_effective = k.checked_add(1).unwrap()
        .checked_mul(price_fix_duration).ok_or(TakeBuyOfferErrorCode::OverflowError)?;

    // Compute: price = P0 * (1 + y * elapsed_effective / S)
    // With fixed-point:
    //   factor_num = SCALE*S + y_scaled*elapsed_effective
    //   factor_den = SCALE*S
    //   price = start_price * factor_num / factor_den
    let factor_den = (SCALE)
        .checked_mul(S as u128)
        .expect("SCALE*S overflow (should not happen)");
    let y_part = (price_yield as u128)
        .checked_mul(elapsed_effective as u128)
        .ok_or(TakeBuyOfferErrorCode::OverflowError)?;
    let factor_num = factor_den
        .checked_add(y_part)
        .ok_or(TakeBuyOfferErrorCode::OverflowError)?;

    // base price growth applied to start_price
    let price_u128 = (start_price as u128)
        .checked_mul(factor_num).ok_or(TakeBuyOfferErrorCode::OverflowError)?
        .checked_div(factor_den).ok_or(TakeBuyOfferErrorCode::OverflowError)?;

    if price_u128 > u64::MAX as u128 {
        return Err(error!(TakeBuyOfferErrorCode::OverflowError));
    }

    Ok(price_u128 as u64)
}

fn find_offer(offer_account: &BuyOfferAccount, offer_id: u64) -> Result<BuyOffer> {
    if (offer_id == 0) {
        return Err(error!(TakeBuyOfferErrorCode::OfferNotFound));
    }
    
    let offer = offer_account.offers
        .iter()
        .find(|offer| offer.offer_id == offer_id)
        .ok_or(TakeBuyOfferErrorCode::OfferNotFound)?;
    
    Ok(*offer)
}

fn find_active_vector(offer: &BuyOffer) -> Result<BuyOfferVector> {
    let current_time = Clock::get()?.unix_timestamp as u64;
    
    let active_vector = offer.vectors
        .iter()
        .filter(|vector| vector.vector_id != 0) // Only consider non-empty vectors
        .filter(|vector| vector.valid_from <= current_time) // Only vectors that have started
        .max_by_key(|vector| vector.valid_from) // Find latest valid_from in the past
        .ok_or(TakeBuyOfferErrorCode::NoActiveVector)?;
    
    Ok(*active_vector)
}

/// Executes both token transfers (user to boss, vault to user)
fn execute_transfers(
    ctx: &Context<TakeBuyOffer>,
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