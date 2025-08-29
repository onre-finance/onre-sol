use crate::constants::seeds;
use crate::instructions::{BuyOffer, BuyOfferAccount, BuyOfferVector};
use crate::utils::{calculate_token_out_amount, transfer_tokens};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

/// Common error codes that can be used by both take_buy_offer instructions
#[error_code]
pub enum BuyOfferCoreError {
    #[msg("Offer not found")]
    OfferNotFound,
    #[msg("No active vector")]
    NoActiveVector,
    #[msg("Overflow error")]
    OverflowError,
}

/// Result structure for the core buy offer processing
pub struct BuyOfferProcessResult {
    pub current_price: u64,
    pub token_out_amount: u64,
}

/// Core processing logic shared between both take_buy_offer instructions
///
/// This function handles all the common validation and calculation logic:
/// 1. Find and validate the offer exists
/// 2. Find the currently active pricing vector
/// 3. Calculate current price based on time and APR parameters
/// 4. Calculate token_out_amount based on price and decimals
///
/// # Arguments
/// * `offer_account` - The loaded buy offer account
/// * `offer_id` - The ID of the offer to process
/// * `token_in_amount` - Amount of token_in being provided
/// * `token_in_mint` - The token_in mint for decimal information
/// * `token_out_mint` - The token_out mint for decimal information
///
/// # Returns
/// A `BuyOfferProcessResult` containing the calculated price and token_out_amount
pub fn process_buy_offer_core(
    offer_account: &BuyOfferAccount,
    offer_id: u64,
    token_in_amount: u64,
    token_in_mint: &Account<Mint>,
    token_out_mint: &Account<Mint>,
) -> Result<BuyOfferProcessResult> {
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Find the offer
    let offer = find_offer(offer_account, offer_id)?;

    // Find the currently active pricing vector
    let active_vector = find_active_vector_at(&offer, current_time)?;

    // Calculate current price with 9 decimals
    let current_price = calculate_current_price(
        active_vector.apr,
        active_vector.start_price,
        active_vector.start_time,
        active_vector.price_fix_duration,
    )?;

    // Calculate how many token_out to give for the provided token_in_amount
    let token_out_amount = calculate_token_out_amount(
        token_in_amount,
        current_price,
        token_in_mint.decimals,
        token_out_mint.decimals,
    )?;

    Ok(BuyOfferProcessResult {
        current_price,
        token_out_amount,
    })
}

/// Finds a buy offer by ID in the offer account
///
/// # Arguments
/// * `offer_account` - The buy offer account containing all offers
/// * `offer_id` - The ID of the offer to find
///
/// # Returns
/// The found `BuyOffer` or an error if not found
pub fn find_offer(offer_account: &BuyOfferAccount, offer_id: u64) -> Result<BuyOffer> {
    if offer_id == 0 {
        return Err(error!(BuyOfferCoreError::OfferNotFound));
    }

    let offer = offer_account
        .offers
        .iter()
        .find(|offer| offer.offer_id == offer_id)
        .ok_or(BuyOfferCoreError::OfferNotFound)?;

    Ok(*offer)
}

/// Finds a mutable reference to buy offer by ID in the offer account
///
/// # Arguments
/// * `offer_account` - The buy offer account containing all offers
/// * `offer_id` - The ID of the offer to find
///
/// # Returns
/// The found `BuyOffer` or an error if not found
pub fn find_offer_mut(offer_account: &mut BuyOfferAccount, offer_id: u64) -> Result<&mut BuyOffer> {
    if offer_id == 0 {
        return Err(error!(BuyOfferCoreError::OfferNotFound));
    }

    let offer = offer_account
        .offers
        .iter_mut()
        .find(|offer| offer.offer_id == offer_id)
        .ok_or(BuyOfferCoreError::OfferNotFound)?;

    Ok(offer)
}

/// Finds a buy offer index by ID in the offer account
///
/// # Arguments
/// * `offer_account` - The buy offer account containing all offers
/// * `offer_id` - The ID of the offer to find
///
/// # Returns
/// The found `BuyOffer` or an error if not found
pub fn find_offer_index(offer_account: &BuyOfferAccount, offer_id: u64) -> Result<usize> {
    if offer_id == 0 {
        return Err(error!(BuyOfferCoreError::OfferNotFound));
    }

    let offer_id = offer_account
        .offers
        .iter()
        .position(|offer| offer.offer_id == offer_id)
        .ok_or(BuyOfferCoreError::OfferNotFound)?;

    Ok(offer_id)
}

/// Finds the currently active vector for a buy offer
///
/// # Arguments
/// * `offer` - The buy offer to search for an active vector
///
/// # Returns
/// The active `BuyOfferVector` or an error if none is active
pub fn find_active_vector_at(offer: &BuyOffer, time: u64) -> Result<BuyOfferVector> {
    let active_vector = offer
        .vectors
        .iter()
        .filter(|vector| vector.vector_id != 0) // Only consider non-empty vectors
        .filter(|vector| vector.valid_from <= time) // Only vectors that have started
        .max_by_key(|vector| vector.valid_from) // Find latest valid_from in the past
        .ok_or(BuyOfferCoreError::NoActiveVector)?;

    Ok(*active_vector)
}

/// Linear (uncompounded) price calculation with "price-fix" windows that snap to the END of the current window.
///
/// This function implements the discrete interval pricing model:
/// - apr: Annual Percentage Rate scaled by 1_000_000 (e.g., 10.12% => 101_200)
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
/// # Arguments
/// * `apr` - Annual Percentage Rate scaled by 1_000_000
/// * `start_price` - Starting price
/// * `start_time` - Unix timestamp when pricing starts
/// * `price_fix_duration` - Duration of each price interval in seconds
///
/// # Returns
/// The calculated current price
pub fn calculate_current_price(
    apr: u64,
    start_price: u64,
    start_time: u64,
    price_fix_duration: u64,
) -> Result<u64> {
    const SCALE: u128 = 1_000_000; // because APR is scaled by 1_000_000
    const S: u64 = 365 * 24 * 3600; // seconds per year

    let current_time = Clock::get()?.unix_timestamp as u64;

    require!(
        start_time <= current_time,
        BuyOfferCoreError::NoActiveVector
    );

    let elapsed_since_start = current_time.saturating_sub(start_time);

    // Calculate which price interval we're in (discrete intervals)
    let k = elapsed_since_start / price_fix_duration;

    // elapsed_effective = (k + 1) * D  (end-of-current-interval snap)
    let elapsed_effective = k
        .checked_add(1)
        .unwrap()
        .checked_mul(price_fix_duration)
        .ok_or(BuyOfferCoreError::OverflowError)?;

    // Compute: price = P0 * (1 + y * elapsed_effective / S)
    // With fixed-point:
    //   factor_num = SCALE*S + y_scaled*elapsed_effective
    //   factor_den = SCALE*S
    //   price = start_price * factor_num / factor_den
    let factor_den = SCALE
        .checked_mul(S as u128)
        .expect("SCALE*S overflow (should not happen)");
    let y_part = (apr as u128)
        .checked_mul(elapsed_effective as u128)
        .ok_or(BuyOfferCoreError::OverflowError)?;
    let factor_num = factor_den
        .checked_add(y_part)
        .ok_or(BuyOfferCoreError::OverflowError)?;

    // base price growth applied to start_price
    let price_u128 = (start_price as u128)
        .checked_mul(factor_num)
        .ok_or(BuyOfferCoreError::OverflowError)?
        .checked_div(factor_den)
        .ok_or(BuyOfferCoreError::OverflowError)?;

    if price_u128 > u64::MAX as u128 {
        return Err(error!(BuyOfferCoreError::OverflowError));
    }

    Ok(price_u128 as u64)
}

/// Execute direct token transfers: User → Boss, Vault → User
pub fn execute_direct_transfers<'info>(
    user: &Signer<'info>,
    user_token_in_account: &Account<'info, TokenAccount>,
    boss_token_in_account: &Account<'info, TokenAccount>,
    vault_authority: &UncheckedAccount<'info>,
    vault_token_out_account: &Account<'info, TokenAccount>,
    user_token_out_account: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    vault_authority_bump: u8,
    token_in_amount: u64,
    token_out_amount: u64,
) -> Result<()> {
    // Transfer token_in from user to boss
    transfer_tokens(
        token_program,
        user_token_in_account,
        boss_token_in_account,
        user,
        None,
        token_in_amount,
    )?;

    // Transfer token_out from vault to user using vault authority
    let vault_authority_seeds = &[seeds::VAULT_AUTHORITY, &[vault_authority_bump]];
    let signer_seeds = &[vault_authority_seeds.as_slice()];

    transfer_tokens(
        token_program,
        vault_token_out_account,
        user_token_out_account,
        vault_authority,
        Some(signer_seeds),
        token_out_amount,
    )?;

    Ok(())
}

/// Execute permissionless token transfers through intermediary accounts
/// 1. User → Permissionless intermediary (token_in)
/// 2. Permissionless intermediary → Boss (token_in)
/// 3. Vault → Permissionless intermediary (token_out)
/// 4. Permissionless intermediary → User (token_out)
pub fn execute_permissionless_transfers<'info>(
    user: &Signer<'info>,
    user_token_in_account: &Account<'info, TokenAccount>,
    boss_token_in_account: &Account<'info, TokenAccount>,
    vault_authority: &UncheckedAccount<'info>,
    vault_token_out_account: &Account<'info, TokenAccount>,
    user_token_out_account: &Account<'info, TokenAccount>,
    permissionless_authority: &UncheckedAccount<'info>,
    permissionless_token_in_account: &Account<'info, TokenAccount>,
    permissionless_token_out_account: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    vault_authority_bump: u8,
    permissionless_authority_bump: u8,
    token_in_amount: u64,
    token_out_amount: u64,
) -> Result<()> {
    let permissionless_authority_seeds =
        &[seeds::PERMISSIONLESS_1, &[permissionless_authority_bump]];
    let permissionless_signer_seeds = &[permissionless_authority_seeds.as_slice()];

    let vault_authority_seeds = &[seeds::VAULT_AUTHORITY, &[vault_authority_bump]];
    let vault_signer_seeds = &[vault_authority_seeds.as_slice()];

    // 1. Transfer token_in from user to permissionless intermediary
    transfer_tokens(
        token_program,
        user_token_in_account,
        permissionless_token_in_account,
        user,
        None,
        token_in_amount,
    )?;

    // 2. Transfer token_in from permissionless intermediary to boss
    transfer_tokens(
        token_program,
        permissionless_token_in_account,
        boss_token_in_account,
        permissionless_authority,
        Some(permissionless_signer_seeds),
        token_in_amount,
    )?;

    // 3. Transfer token_out from vault to permissionless intermediary
    transfer_tokens(
        token_program,
        vault_token_out_account,
        permissionless_token_out_account,
        vault_authority,
        Some(vault_signer_seeds),
        token_out_amount,
    )?;

    // 4. Transfer token_out from permissionless intermediary to user
    transfer_tokens(
        token_program,
        permissionless_token_out_account,
        user_token_out_account,
        permissionless_authority,
        Some(permissionless_signer_seeds),
        token_out_amount,
    )?;

    Ok(())
}
