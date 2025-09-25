use crate::constants::seeds;
use crate::instructions::offer::offer_utils::{
    calculate_current_step_price, find_active_vector_at, find_offer,
};
use crate::instructions::OfferAccount;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;

use crate::utils::PRICE_DECIMALS;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[error_code]
pub enum GetTVLErrorCode {
    #[msg("Math overflow")]
    Overflow,
    #[msg("Invalid token_out vault account")]
    InvalidVaultAccount,
}

/// Event emitted when get_TVL is called
#[event]
pub struct GetTVLEvent {
    /// The ID of the offer
    pub offer_id: u64,
    /// Current TVL for the offer
    pub tvl: u64,
    /// Current price used for TVL calculation
    pub current_price: u64,
    /// Token supply used for TVL calculation
    pub token_supply: u64,
    /// Unix timestamp when the TVL was calculated
    pub timestamp: u64,
}

/// Accounts required for getting TVL information
#[derive(Accounts)]
pub struct GetTVL<'info> {
    /// The offer account containing all active offers
    #[account(seeds = [seeds::OFFERS], bump)]
    pub offer_account: AccountLoader<'info, OfferAccount>,

    /// The token_out mint account to get supply information
    pub token_out_mint: InterfaceAccount<'info, Mint>,

    /// The offer vault authority PDA that controls vault token accounts
    /// CHECK: This is safe as it's a PDA
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    /// The token_out account to exclude from supply
    /// CHECK: This account is validated by the check below to allow passing uninitialized vault account
    #[account(
        // enforce the exact ATA address
        constraint = vault_token_out_account.key()
            == get_associated_token_address_with_program_id(
                &vault_authority.key(),
                &token_out_mint.key(),
                &token_out_program.key(),
            ) @ GetTVLErrorCode::InvalidVaultAccount
    )]
    pub vault_token_out_account: UncheckedAccount<'info>,

    pub token_out_program: Interface<'info, TokenInterface>,
}

/// Gets the current TVL (Total Value Locked) for a specific offer
///
/// This instruction allows anyone to query the current TVL for an offer
/// without making any state modifications. The TVL is calculated as:
/// TVL = token_out_supply * current_NAV
///
/// # Arguments
///
/// * `ctx` - The instruction context containing required accounts
/// * `offer_id` - The unique ID of the offer to get the TVL for
///
/// # Returns
///
/// * `Ok(tvl)` - If the TVL was successfully calculated
/// * `Err(_)` - If the offer doesn't exist, TVL calculation fails, or math overflow occurs
///
/// # Emits
///
/// * `GetTVLEvent` - Contains offer_id, tvl, current_price, token_supply, and timestamp
pub fn get_tvl(ctx: Context<GetTVL>, offer_id: u64) -> Result<u64> {
    let offer_account = ctx.accounts.offer_account.load()?;
    let current_time = Clock::get()?.unix_timestamp as u64;

    // Find the offer
    let offer = find_offer(&offer_account, offer_id)?;

    // Verify that the provided token_out_mint matches the offer's token_out_mint
    require_eq!(
        ctx.accounts.token_out_mint.key(),
        offer.token_out_mint,
        ErrorCode::ConstraintHasOne
    );

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
        "TVL Info - Offer ID: {}, TVL: {}, Current Price: {}, Token Supply: {}, Timestamp: {}",
        offer_id,
        tvl,
        current_price,
        token_supply,
        current_time
    );

    emit!(GetTVLEvent {
        offer_id,
        tvl,
        current_price,
        token_supply,
        timestamp: current_time,
    });

    Ok(tvl)
}

/// Read amount from an ATA only if it’s initialized under the given token program.
/// Returns Ok(0) if the account is uninitialized or not a token account yet.
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
