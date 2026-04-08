use crate::constants::seeds;
use crate::instructions::market_info::market_stats::{
    calculate_circulating_supply, calculate_tvl as calculate_shared_tvl,
};
use crate::instructions::market_info::offer_valuation_utils::get_active_vector_and_current_price;
use crate::instructions::Offer;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;

use crate::utils::read_optional_token_account_amount;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::{Mint, TokenInterface};

// Error codes for TVL calculation operations

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
            @ crate::OnreError::InvalidTokenInMint
    )]
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    /// The output token mint account containing total supply information
    #[account(
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ crate::OnreError::InvalidTokenOutMint
    )]
    pub token_out_mint: InterfaceAccount<'info, Mint>,

    /// The vault authority PDA that controls vault token accounts
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub vault_authority: UncheckedAccount<'info>,

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
            ) @ crate::OnreError::InvalidVaultAccount
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
/// * `Err(crate::OnreError::NoActiveVector)` - If no pricing vector is currently active
/// * `Err(crate::OnreError::Overflow)` - If mathematical overflow occurs during calculation
/// * `Err(crate::OnreError::InvalidVaultAccount)` - If vault account validation fails
///
/// # Events
/// * `GetTVLEvent` - Emitted with TVL, price, supply, and timestamp details
pub fn get_tvl(ctx: Context<GetTVL>) -> Result<u64> {
    let offer = ctx.accounts.offer.load()?;
    let current_time = Clock::get()?.unix_timestamp as u64;

    let (_, current_price) = get_active_vector_and_current_price(&offer, current_time)?;

    let vault_token_out_amount = read_optional_token_account_amount(
        &ctx.accounts.vault_token_out_account,
        &ctx.accounts.token_out_program,
    )?;

    // Get token supply
    let token_supply =
        calculate_circulating_supply(ctx.accounts.token_out_mint.supply, vault_token_out_amount);

    // Calculate TVL = supply * price
    // Both supply and price should be compatible for multiplication
    let tvl = calculate_shared_tvl(token_supply, current_price)
        .map_err(|_| error!(crate::OnreError::Overflow))?;

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
