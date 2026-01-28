use crate::constants::seeds;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;

use crate::state::State;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Error codes for circulating supply calculation operations
#[error_code]
pub enum GetCirculatingSupplyErrorCode {
    /// The vault account address doesn't match the expected ATA address
    #[msg("Invalid token_out vault account")]
    InvalidVaultAccount,
}

/// Event emitted when circulating supply calculation is completed
///
/// Provides transparency for tracking token supply distribution and vault holdings.
#[event]
pub struct GetCirculatingSupplyEvent {
    /// Calculated circulating supply (total_supply - vault_amount) in base units
    pub circulating_supply: u64,
    /// Total token supply from the mint account in base units
    pub total_supply: u64,
    /// Vault token amount excluded from circulation in base units
    pub vault_amount: u64,
    /// Unix timestamp when the calculation was performed
    pub timestamp: u64,
}

/// Account structure for querying circulating supply information
///
/// This struct defines the accounts required to calculate the circulating supply
/// of ONyc tokens by subtracting vault holdings from total supply. All accounts
/// are validated to ensure accurate calculation.
#[derive(Accounts)]
pub struct GetCirculatingSupply<'info> {
    /// The ONyc token mint containing total supply information
    pub onyc_mint: InterfaceAccount<'info, Mint>,

    /// Program state account containing the ONyc mint reference
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = onyc_mint)]
    pub state: Box<Account<'info, State>>,

    /// The vault authority PDA that controls vault token accounts
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    /// The vault's ONyc token account to exclude from circulating supply
    ///
    /// This account holds tokens that are not considered in circulation.
    /// The account address is validated to match the expected ATA address
    /// and can be uninitialized (treated as zero balance).
    /// CHECK: Account address is validated by the constraint below to allow passing uninitialized vault account
    #[account(
        constraint = onyc_vault_account.key()
            == get_associated_token_address_with_program_id(
                &vault_authority.key(),
                &state.onyc_mint.key(),
                &token_program.key(),
            ) @ GetCirculatingSupplyErrorCode::InvalidVaultAccount
    )]
    pub onyc_vault_account: UncheckedAccount<'info>,

    /// SPL Token program for account validation
    pub token_program: Interface<'info, TokenInterface>,
}

/// Calculates and returns the current circulating supply of ONyc tokens
///
/// This read-only instruction calculates the circulating supply by subtracting
/// vault holdings from the total token supply. The vault amount represents tokens
/// held by the program that are not considered in active circulation.
///
/// Formula: `circulating_supply = total_supply - vault_amount`
///
/// The vault account can be uninitialized (treated as zero balance) or contain
/// tokens that should be excluded from circulation calculations.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(circulating_supply)` - The calculated circulating supply in base units
/// * `Err(GetCirculatingSupplyErrorCode::InvalidVaultAccount)` - If vault account validation fails
///
/// # Events
/// * `GetCirculatingSupplyEvent` - Emitted with calculation details and timestamp
pub fn get_circulating_supply(ctx: Context<GetCirculatingSupply>) -> Result<u64> {
    let current_time = Clock::get()?.unix_timestamp as u64;

    let vault_token_out_amount = read_optional_ata_amount(
        &ctx.accounts.onyc_vault_account,
        &ctx.accounts.token_program,
    )?;

    // Get total supply
    let total_supply = ctx.accounts.onyc_mint.supply;

    // Calculate circulating supply = total supply - vault amount
    let circulating_supply = total_supply - vault_token_out_amount;

    msg!(
        "Circulating Supply Info - Circulating Supply: {}, Total Supply: {}, Vault Amount: {}, Timestamp: {}",
        circulating_supply,
        total_supply,
        vault_token_out_amount,
        current_time
    );

    emit!(GetCirculatingSupplyEvent {
        circulating_supply,
        total_supply,
        vault_amount: vault_token_out_amount,
        timestamp: current_time,
    });

    Ok(circulating_supply)
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
fn read_optional_ata_amount<'info>(
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
