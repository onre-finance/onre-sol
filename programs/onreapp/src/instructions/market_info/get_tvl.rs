use crate::constants::{seeds, PRICE_DECIMALS};
use crate::instructions::offer::offer_utils::{
    calculate_current_step_price, find_active_vector_at,
};
use crate::instructions::Offer;
use crate::OfferCoreError;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;

use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Error codes for TVL calculation operations
#[error_code]
pub enum GetTVLErrorCode {
    /// Mathematical overflow during TVL calculations
    #[msg("Math overflow")]
    Overflow,
    /// The vault account address doesn't match the expected ATA address
    #[msg("Invalid token_out vault account")]
    InvalidVaultAccount,
}

/// Event emitted when TVL (Total Value Locked) calculation is completed
///
/// Provides transparency for tracking total value metrics for offers.
#[event]
pub struct GetTVLEvent {
    /// The PDA address of the offer for which TVL was calculated
    pub offer_pda: Pubkey,
    /// Calculated TVL in base units (circulating_supply * current_price / 10^9)
    pub tvl: u64,
    /// Current price with scale=9 used for TVL calculation
    pub current_price: u64,
    /// Circulating token supply (total_supply - vault_amount) in base units
    pub token_supply: u64,
    /// Unix timestamp when the TVL calculation was performed
    pub timestamp: u64,
}

/// Account structure for querying TVL (Total Value Locked) information
///
/// This struct defines the accounts required to calculate the TVL for a specific
/// offer by combining current pricing with circulating token supply. The calculation
/// is read-only and validates all accounts belong to the same offer.
#[derive(Accounts)]
pub struct GetTVL<'info> {
    /// The offer account containing pricing vectors for current price calculation
    ///
    /// This account is validated as a PDA derived from token mint addresses
    /// and contains time-based pricing vectors for TVL calculation.
    #[account(
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump = offer.load()?.bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// The input token mint account for offer validation
    #[account(
        constraint =
            token_in_mint.key() == offer.load()?.token_in_mint
            @ OfferCoreError::InvalidTokenInMint
    )]
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    /// The output token mint account containing total supply information
    #[account(
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ OfferCoreError::InvalidTokenOutMint
    )]
    pub token_out_mint: InterfaceAccount<'info, Mint>,

    /// The vault authority PDA that controls vault token accounts
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub vault_authority: AccountInfo<'info>,

    /// The vault's token_out account to exclude from circulating supply
    ///
    /// This account holds tokens that should not be included in TVL calculations.
    /// The account address is validated to match the expected ATA address
    /// and can be uninitialized (treated as zero balance).
    /// CHECK: Account address is validated by the constraint below to allow passing uninitialized vault account
    #[account(
        constraint = vault_token_out_account.key()
            == get_associated_token_address_with_program_id(
                &vault_authority.key(),
                &token_out_mint.key(),
                &token_out_program.key(),
            ) @ GetTVLErrorCode::InvalidVaultAccount
    )]
    pub vault_token_out_account: UncheckedAccount<'info>,

    /// SPL Token program for vault account validation
    pub token_out_program: Interface<'info, TokenInterface>,
}

/// Calculates and returns the current TVL (Total Value Locked) for a specific offer
///
/// This read-only instruction calculates the TVL by combining the current NAV price
/// with the circulating token supply. The calculation excludes vault holdings from
/// the total supply to represent only tokens in circulation.
///
/// Formula: `TVL = circulating_supply * current_price / 10^9`
///
/// The calculation uses the current active pricing vector to determine NAV and
/// subtracts vault holdings from total supply to get circulating supply.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(tvl)` - The calculated TVL in base units
/// * `Err(OfferCoreError::NoActiveVector)` - If no pricing vector is currently active
/// * `Err(GetTVLErrorCode::Overflow)` - If mathematical overflow occurs during calculation
/// * `Err(GetTVLErrorCode::InvalidVaultAccount)` - If vault account validation fails
///
/// # Events
/// * `GetTVLEvent` - Emitted with TVL, price, supply, and timestamp details
pub fn get_tvl(ctx: Context<GetTVL>) -> Result<u64> {
    let offer = ctx.accounts.offer.load()?;
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Find the currently active pricing vector
    let active_vector = find_active_vector_at(&offer, current_time)?;

    // Calculate current price (NAV) with 9 decimals
    let current_price = calculate_current_step_price(
        active_vector.apr,
        active_vector.base_price,
        active_vector.base_time,
        active_vector.price_fix_duration,
    )?;

    let vault_token_out_amount = read_optional_ata_amount(
        &ctx.accounts.vault_token_out_account,
        &ctx.accounts.token_out_program,
    )?;

    // Get token supply
    let token_supply = ctx.accounts.token_out_mint.supply - vault_token_out_amount;

    // Calculate TVL = supply * price
    // Both supply and price should be compatible for multiplication
    let tvl = (token_supply as u128)
        .checked_mul(current_price as u128)
        .and_then(|result| {
            // Since price has 9 decimals, we divide by 1e9 to get the actual TVL
            result.checked_div(10_u128.pow(PRICE_DECIMALS as u32))
        })
        .and_then(|result| {
            if result <= u64::MAX as u128 {
                Some(result as u64)
            } else {
                None
            }
        })
        .ok_or(GetTVLErrorCode::Overflow)?;

    msg!(
        "TVL Info - Offer PDA: {}, TVL: {}, Current Price: {}, Token Supply: {}, Timestamp: {}",
        ctx.accounts.offer.key(),
        tvl,
        current_price,
        token_supply,
        current_time
    );

    emit!(GetTVLEvent {
        offer_pda: ctx.accounts.offer.key(),
        tvl,
        current_price,
        token_supply,
        timestamp: current_time,
    });

    Ok(tvl)
}

/// Safely reads token amount from an Associated Token Account
///
/// This function handles both initialized and uninitialized token accounts,
/// returning zero for accounts that don't exist or aren't properly initialized.
/// Supports both Token and Token-2022 programs with extension handling.
///
/// # Arguments
/// * `vault_account` - The token account to read from
/// * `token_program` - The SPL Token program for ownership validation
///
/// # Returns
/// * `Ok(amount)` - Token amount if account is initialized, 0 otherwise
fn read_optional_ata_amount(
    vault_account: &AccountInfo,
    token_program: &Interface<TokenInterface>,
) -> Result<u64> {
    // If it’s not owned by the token program, it’s not initialized (likely System Program)
    if vault_account.owner != token_program.key {
        return Ok(0);
    }

    // If there’s no data, treat as uninitialized.
    if vault_account.data_is_empty() {
        return Ok(0);
    }

    // Try to deserialize as a TokenInterface account; if this fails, treat as 0.
    // (Token-2022 accounts can be larger due to extensions; try_deserialize handles it.)
    let data_ref = vault_account.data.borrow();
    match TokenAccount::try_deserialize(&mut &data_ref[..]) {
        Ok(parsed) => Ok(parsed.amount),
        Err(_) => Ok(0),
    }
}
