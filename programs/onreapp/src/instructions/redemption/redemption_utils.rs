use crate::constants::seeds;
use crate::instructions::{calculate_current_step_price, find_active_vector_at, Offer};
use crate::utils::{burn_tokens, mint_tokens, transfer_tokens};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Common error codes for redemption processing operations
#[error_code]
pub enum RedemptionCoreError {
    /// No pricing vector is currently active for the given time
    #[msg("No active vector")]
    NoActiveVector,
    /// Arithmetic overflow occurred during calculations
    #[msg("Overflow error")]
    OverflowError,
}

/// Result structure containing redemption processing calculations
pub struct RedemptionProcessResult {
    /// Inverted price with scale=9 (1_000_000_000 = 1.0) at the time of processing
    /// This is the inverted price from the offer's perspective for redemption context
    pub inverted_price: u64,
    /// Calculated amount of token_out to be provided to the user
    pub token_out_amount: u64,
}

/// Core processing logic for redemption execution calculations
///
/// Performs price inversion and token amount calculation for redemption offers.
/// The underlying offer has price "X token_out per token_in" (e.g., "2 USDC per ONyc"),
/// but for redemption we need to invert this to get "Y token_in per token_out".
///
/// # Arguments
/// * `offer` - The underlying offer containing pricing vectors and configuration
/// * `token_in_amount` - Amount of token_in being redeemed by the user
/// * `token_in_mint` - The token_in mint for decimal information (what user is redeeming)
/// * `token_out_mint` - The token_out mint for decimal information (what user receives)
///
/// # Returns
/// * `Ok(RedemptionProcessResult)` - Containing inverted price and token_out amount
/// * `Err(_)` - If validation fails or no active vector exists
///
/// # Price Inversion
/// The offer's price is inverted using: `inverted_price = 10^18 / price`
/// This maintains 9 decimal precision after inverting a 9-decimal price.
///
/// # Example
/// - Offer price: 2.0 USDC per ONyc (2_000_000_000 with 9 decimals)
/// - Inverted: 0.5 ONyc per USDC (500_000_000 with 9 decimals)
/// - User redeems: 10 ONyc
/// - User receives: 20 USDC (10 ONyc * 2.0 USDC/ONyc)
pub fn process_redemption_core(
    offer: &Offer,
    token_in_amount: u64,
    token_in_mint: &InterfaceAccount<Mint>,
    token_out_mint: &InterfaceAccount<Mint>,
) -> Result<RedemptionProcessResult> {
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Find the currently active pricing vector
    let active_vector = find_active_vector_at(offer, current_time)?;

    // Calculate current price with 9 decimals
    let current_price = calculate_current_step_price(
        active_vector.apr,
        active_vector.base_price,
        active_vector.base_time,
        active_vector.price_fix_duration,
    )?;

    // Invert the price to get the correct redemption calculation
    // price has 9 decimals, so inverted_price = 10^18 / price maintains 9 decimals
    const PRICE_SCALE: u128 = 1_000_000_000; // 10^9
    let price_u128 = current_price as u128;
    let inverted_price_u128 = (PRICE_SCALE * PRICE_SCALE)
        .checked_div(price_u128)
        .ok_or(RedemptionCoreError::OverflowError)?;
    let inverted_price = inverted_price_u128 as u64;

    // Calculate token_out using the inverted price
    // token_out_amount = (token_in_amount * 10^(token_out_decimals + 9)) / (inverted_price * 10^token_in_decimals)
    let token_in_amount_u128 = token_in_amount as u128;
    let numerator = token_in_amount_u128
        .checked_mul(10_u128.pow((token_out_mint.decimals + 9) as u32))
        .ok_or(RedemptionCoreError::OverflowError)?;
    let denominator = inverted_price_u128
        .checked_mul(10_u128.pow(token_in_mint.decimals as u32))
        .ok_or(RedemptionCoreError::OverflowError)?;
    let token_out_amount = (numerator / denominator) as u64;

    Ok(RedemptionProcessResult {
        inverted_price,
        token_out_amount,
    })
}

/// Parameters for executing redemption token operations
///
/// This structure contains all the accounts and parameters needed to execute
/// a complete redemption token exchange, handling token_in burning/transfer
/// and token_out minting/transfer based on mint authority.
pub struct ExecuteRedemptionOpsParams<'a, 'info> {
    /// SPL Token program for token_in operations
    pub token_in_program: &'a Interface<'info, TokenInterface>,
    /// SPL Token program for token_out operations
    pub token_out_program: &'a Interface<'info, TokenInterface>,

    // Token in params (what user is redeeming)
    /// Mint account for the input token
    pub token_in_mint: &'a InterfaceAccount<'info, Mint>,
    /// Amount of token_in to process
    pub token_in_amount: u64,
    /// Vault account containing locked token_in
    pub vault_token_in_account: &'a InterfaceAccount<'info, TokenAccount>,
    /// Boss's account for receiving token_in when program lacks mint authority
    pub boss_token_in_account: &'a InterfaceAccount<'info, TokenAccount>,
    /// Authority for vault operations
    pub redemption_vault_authority: &'a AccountInfo<'info>,
    /// Bump seed for vault authority
    pub redemption_vault_authority_bump: u8,

    // Token out params (what user receives)
    /// Mint account for the output token
    pub token_out_mint: &'a InterfaceAccount<'info, Mint>,
    /// Amount of token_out to distribute
    pub token_out_amount: u64,
    /// Vault account for token_out distribution
    pub vault_token_out_account: &'a InterfaceAccount<'info, TokenAccount>,
    /// User's account for receiving token_out
    pub user_token_out_account: &'a InterfaceAccount<'info, TokenAccount>,

    // Mint authority params
    /// PDA for mint authority operations
    pub mint_authority_pda: &'a AccountInfo<'info>,
    /// Bump seed for mint authority PDA
    pub mint_authority_bump: u8,

    // State params
    /// Maximum supply cap for token_out minting (0 = no cap)
    pub token_out_max_supply: u64,
}

/// Executes token operations for redemption
///
/// This function handles the complete redemption token exchange process with intelligent
/// routing based on mint authority ownership.
///
/// # Token In Processing (already locked in vault)
/// - If token_in is ONyc AND program has mint authority: burn it from vault
/// - Otherwise: transfer from vault to boss account
///
/// # Token Out Processing
/// - If program has mint authority: mint directly to user
/// - If program lacks mint authority: transfer from vault to user
///
/// # Arguments
/// * `params` - Complete parameter structure containing all required accounts and amounts
///
/// # Returns
/// * `Ok(())` - If all token operations complete successfully
/// * `Err(_)` - If any transfer, mint, or burn operation fails
pub fn execute_redemption_operations(params: ExecuteRedemptionOpsParams) -> Result<()> {
    let vault_authority_signer_seeds: &[&[&[u8]]] = &[&[
        seeds::REDEMPTION_OFFER_VAULT_AUTHORITY,
        &[params.redemption_vault_authority_bump],
    ]];

    // Step 1: Handle token_in (burn or transfer to boss)
    let has_token_in_mint_authority = params.token_in_mint.mint_authority
        .as_ref()
        .map(|auth| auth == &params.mint_authority_pda.key())
        .unwrap_or(false);

    if has_token_in_mint_authority {
        // Burn token_in from vault
        burn_tokens(
            params.token_in_program,
            params.token_in_mint,
            params.vault_token_in_account,
            params.redemption_vault_authority,
            vault_authority_signer_seeds,
            params.token_in_amount,
        )?;
    } else {
        // Transfer token_in from vault to boss
        transfer_tokens(
            params.token_in_mint,
            params.token_in_program,
            params.vault_token_in_account,
            params.boss_token_in_account,
            params.redemption_vault_authority,
            Some(vault_authority_signer_seeds),
            params.token_in_amount,
        )?;
    }

    // Step 2: Distribute token_out to user
    let has_token_out_mint_authority = params.token_out_mint.mint_authority
        .as_ref()
        .map(|auth| auth == &params.mint_authority_pda.key())
        .unwrap_or(false);

    if has_token_out_mint_authority {
        // Mint token_out directly to user
        let mint_authority_signer_seeds: &[&[&[u8]]] = &[&[
            seeds::MINT_AUTHORITY,
            &[params.mint_authority_bump],
        ]];

        mint_tokens(
            params.token_out_program,
            params.token_out_mint,
            params.user_token_out_account,
            params.mint_authority_pda,
            mint_authority_signer_seeds,
            params.token_out_amount,
            params.token_out_max_supply,
        )?;
    } else {
        // Transfer token_out from vault to user
        transfer_tokens(
            params.token_out_mint,
            params.token_out_program,
            params.vault_token_out_account,
            params.user_token_out_account,
            params.redemption_vault_authority,
            Some(vault_authority_signer_seeds),
            params.token_out_amount,
        )?;
    }

    Ok(())
}
