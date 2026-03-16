use crate::constants::{seeds, PRICE_DECIMALS};
use crate::instructions::{calculate_current_step_price, find_active_vector_at, Offer};
use crate::utils::{
    burn_tokens, calculate_fees, mint_tokens, program_controls_mint, transfer_tokens,
};
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
    /// Price with scale=9 (1_000_000_000 = 1.0) at the time of processing
    pub price: u64,
    /// Amount of token_in after fee deduction
    pub token_in_net_amount: u64,
    /// Fee amount deducted from the original token_in amount
    pub token_in_fee_amount: u64,
    /// Calculated amount of token_out to be provided to the user
    pub token_out_amount: u64,
}

/// Core processing logic for redemption execution calculations
///
/// Calculates token amount for redemption offers using direct price multiplication.
/// The underlying offer has price "X token_out per token_in" (e.g., "2 USDC per ONyc"),
/// and we multiply the token_in amount by this price to get the token_out amount.
/// Fees are deducted from the token_in amount before calculating token_out.
///
/// # Arguments
/// * `offer` - The underlying offer containing pricing vectors and configuration
/// * `token_in_amount` - Amount of token_in being redeemed by the user
/// * `token_in_mint` - The token_in mint for decimal information (what user is redeeming)
/// * `token_out_mint` - The token_out mint for decimal information (what user receives)
/// * `redemption_fee_basis_points` - Fee in basis points (10000 = 100%)
///
/// # Returns
/// * `Ok(RedemptionProcessResult)` - Containing price, fees, and token_out amount
/// * `Err(_)` - If validation fails or no active vector exists
///
/// # Price Calculation
/// Uses the formula: `token_out = (token_in_net * price * 10^token_out_decimals) / (10^token_in_decimals * 10^9)`
/// Price has 9 decimal places, so we divide by 10^9 to account for this.
/// Fees are calculated as: `fee = token_in_amount * fee_basis_points / 10000`
///
/// # Example
/// - Offer price: 2.0 USDC per ONyc (2_000_000_000 with 9 decimals)
/// - User redeems: 10 ONyc
/// - Fee: 1% (100 basis points) = 0.1 ONyc
/// - Net: 9.9 ONyc
/// - User receives: 19.8 USDC (9.9 ONyc * 2.0 USDC/ONyc)
pub fn process_redemption_core(
    offer: &Offer,
    token_in_amount: u64,
    token_in_mint: &InterfaceAccount<Mint>,
    token_out_mint: &InterfaceAccount<Mint>,
    redemption_fee_basis_points: u16,
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

    // Calculate fees
    let fee_amounts = calculate_fees(token_in_amount, redemption_fee_basis_points)?;

    // Calculate token_out using direct multiplication with price (after fee deduction)
    // token_out_amount = (token_in_net_amount * price * 10^token_out_decimals) / (10^(token_in_decimals + 9))
    // price has 9 decimals, so we need to account for that in our calculation
    let price_u128 = current_price as u128;
    let token_in_net_amount_u128 = fee_amounts.token_in_net_amount as u128;

    let numerator = token_in_net_amount_u128
        .checked_mul(price_u128)
        .ok_or(RedemptionCoreError::OverflowError)?
        .checked_mul(10_u128.pow(token_out_mint.decimals as u32))
        .ok_or(RedemptionCoreError::OverflowError)?;

    let denominator = 10_u128
        .pow(token_in_mint.decimals as u32)
        .checked_mul(10_u128.pow(PRICE_DECIMALS as u32))
        .ok_or(RedemptionCoreError::OverflowError)?;

    let result = numerator / denominator;

    // Validate result fits in u64 before casting
    require!(
        result <= u64::MAX as u128,
        RedemptionCoreError::OverflowError
    );

    let token_out_amount = result as u64;

    Ok(RedemptionProcessResult {
        price: current_price,
        token_in_net_amount: fee_amounts.token_in_net_amount,
        token_in_fee_amount: fee_amounts.token_in_fee_amount,
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
    /// Amount of token_in to process (net amount after fee)
    pub token_in_net_amount: u64,
    /// Fee amount to transfer to boss
    pub token_in_fee_amount: u64,
    /// Vault account containing locked token_in
    pub vault_token_in_account: &'a InterfaceAccount<'info, TokenAccount>,
    /// Boss's account for receiving token_in when program lacks mint authority (or fees)
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
/// - If program has mint authority:
///   1. Burn net amount from vault
///   2. Transfer fee amount to boss (if fee > 0)
/// - If program lacks mint authority:
///   - Transfer full amount (net + fee) from vault to boss
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
    let has_token_in_mint_authority =
        program_controls_mint(params.token_in_mint, params.mint_authority_pda);

    if has_token_in_mint_authority {
        // Burn net amount from vault
        burn_tokens(
            params.token_in_program,
            params.token_in_mint,
            params.vault_token_in_account,
            params.redemption_vault_authority,
            vault_authority_signer_seeds,
            params.token_in_net_amount,
        )?;

        // Transfer fee amount to boss if there is a fee
        if params.token_in_fee_amount > 0 {
            msg!("Transferring fee amount to boss account");
            transfer_tokens(
                params.token_in_mint,
                params.token_in_program,
                params.vault_token_in_account,
                params.boss_token_in_account,
                params.redemption_vault_authority,
                Some(vault_authority_signer_seeds),
                params.token_in_fee_amount,
            )?;
        }
    } else {
        // When program lacks mint authority: transfer full amount (net + fee) to boss
        // Use checked_add to prevent overflow
        let total_amount = params
            .token_in_net_amount
            .checked_add(params.token_in_fee_amount)
            .ok_or(RedemptionCoreError::OverflowError)?;

        transfer_tokens(
            params.token_in_mint,
            params.token_in_program,
            params.vault_token_in_account,
            params.boss_token_in_account,
            params.redemption_vault_authority,
            Some(vault_authority_signer_seeds),
            total_amount,
        )?;
    }

    // Step 2: Distribute token_out to user
    let has_token_out_mint_authority =
        program_controls_mint(params.token_out_mint, params.mint_authority_pda);

    if has_token_out_mint_authority {
        // Mint token_out directly to user
        let mint_authority_signer_seeds: &[&[&[u8]]] =
            &[&[seeds::MINT_AUTHORITY, &[params.mint_authority_bump]]];

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
