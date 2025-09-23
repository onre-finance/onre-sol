use crate::constants::seeds;
use crate::instructions::offer::offer_utils::find_offer;
use crate::instructions::OfferAccount;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;

use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[error_code]
pub enum GetCirculatingSupplyErrorCode {
    #[msg("Invalid token_out vault account")]
    InvalidVaultAccount,
}

/// Event emitted when get_circulating_supply is called
#[event]
pub struct GetCirculatingSupplyEvent {
    /// The ID of the offer
    pub offer_id: u64,
    /// Current circulating supply for the offer
    pub circulating_supply: u64,
    /// Total token supply
    pub total_supply: u64,
    /// Vault token amount (excluded from circulating supply)
    pub vault_amount: u64,
    /// Unix timestamp when the circulating supply was calculated
    pub timestamp: u64,
}

/// Accounts required for getting circulating supply information
#[derive(Accounts)]
pub struct GetCirculatingSupply<'info> {
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
            ) @ GetCirculatingSupplyErrorCode::InvalidVaultAccount
    )]
    pub vault_token_out_account: UncheckedAccount<'info>,

    pub token_out_program: Interface<'info, TokenInterface>,
}

/// Gets the current circulating supply for a specific offer
///
/// This instruction allows anyone to query the current circulating supply for an offer
/// without making any state modifications. The circulating supply is calculated as:
/// circulating_supply = total_supply - vault_amount
///
/// # Arguments
///
/// * `ctx` - The instruction context containing required accounts
/// * `offer_id` - The unique ID of the offer to get the circulating supply for
///
/// # Returns
///
/// * `Ok(circulating_supply)` - If the circulating supply was successfully calculated
/// * `Err(_)` - If the offer doesn't exist or calculation fails
///
/// # Emits
///
/// * `GetCirculatingSupplyEvent` - Contains offer_id, circulating_supply, total_supply, vault_amount, and timestamp
pub fn get_circulating_supply(ctx: Context<GetCirculatingSupply>, offer_id: u64) -> Result<u64> {
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

    let vault_token_out_amount = read_optional_ata_amount(
        &ctx.accounts.vault_token_out_account,
        &ctx.accounts.token_out_program,
    )?;

    // Get total supply
    let total_supply = ctx.accounts.token_out_mint.supply;

    // Calculate circulating supply = total supply - vault amount
    let circulating_supply = total_supply - vault_token_out_amount;

    msg!(
        "Circulating Supply Info - Offer ID: {}, Circulating Supply: {}, Total Supply: {}, Vault Amount: {}, Timestamp: {}",
        offer_id,
        circulating_supply,
        total_supply,
        vault_token_out_amount,
        current_time
    );

    emit!(GetCirculatingSupplyEvent {
        offer_id,
        circulating_supply,
        total_supply,
        vault_amount: vault_token_out_amount,
        timestamp: current_time,
    });

    Ok(circulating_supply)
}

/// Read amount from an ATA only if it's initialized under the given token program.
/// Returns Ok(0) if the account is uninitialized or not a token account yet.
fn read_optional_ata_amount(
    vault_account: &AccountInfo,
    token_program: &Interface<TokenInterface>,
) -> Result<u64> {
    // If it's not owned by the token program, it's not initialized (likely System Program)
    if vault_account.owner != token_program.key {
        return Ok(0);
    }

    // If there's no data, treat as uninitialized.
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