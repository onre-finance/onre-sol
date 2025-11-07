use crate::constants::seeds;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token_interface;
use anchor_spl::token_interface::{
    BurnChecked, Mint, MintToChecked, TokenAccount, TokenInterface, TransferChecked,
};

pub const MAX_BASIS_POINTS: u16 = 10000;
pub const PRICE_DECIMALS: u8 = 9;

#[error_code]
pub enum TokenUtilsErrorCode {
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Minting would exceed maximum supply cap")]
    MaxSupplyExceeded,
}

/// Generic token transfer function that handles both regular and PDA-signed transfers
///
/// # Arguments
/// * `token_program` - The SPL Token program
/// * `from_account` - Source token account
/// * `to_account` - Destination token account  
/// * `authority` - The authority that can transfer from the source account
/// * `signer_seeds` - Optional PDA seeds for program-signed transfers (None for user-signed)
/// * `amount` - Amount of tokens to transfer
pub fn transfer_tokens<'info>(
    mint: &InterfaceAccount<'info, Mint>,
    token_program: &Interface<'info, TokenInterface>,
    from_account: &InterfaceAccount<'info, TokenAccount>,
    to_account: &InterfaceAccount<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    signer_seeds: Option<&[&[&[u8]]]>,
    amount: u64,
) -> Result<()> {
    let transfer_accounts = TransferChecked {
        mint: mint.to_account_info(),
        from: from_account.to_account_info(),
        to: to_account.to_account_info(),
        authority: authority.to_account_info(),
    };

    let cpi_context = match signer_seeds {
        Some(seeds) => {
            CpiContext::new_with_signer(token_program.to_account_info(), transfer_accounts, seeds)
        }
        None => CpiContext::new(token_program.to_account_info(), transfer_accounts),
    };

    token_interface::transfer_checked(cpi_context, amount, mint.decimals)
}

/// Calculates token_out_amount based on token_in_amount, price, and decimals.
/// This formula is used in both single and dual redemption offers.
///
/// Formula: token_out_amount = (token_in_amount * 10^(token_out_decimals + 9)) / (price * 10^token_in_decimals)
///
/// # Arguments
/// * `token_in_amount` - Amount of input tokens
/// * `price` - Price with 9 decimal precision (e.g., 2.0 = 2000000000)
/// * `token_in_decimals` - Decimal places of input token
/// * `token_out_decimals` - Decimal places of output token
///
/// # Returns
/// The calculated amount of output tokens
///
/// # Errors
/// Returns MathOverflow if calculation exceeds u128 limits
pub fn calculate_token_out_amount(
    token_in_amount: u64,
    price: u64,
    token_in_decimals: u8,
    token_out_decimals: u8,
) -> Result<u64> {
    let token_in_amount_u128 = token_in_amount as u128;
    let price_u128 = price as u128;

    // Calculate: numerator = token_in_amount * 10^(token_out_decimals + 9)
    let numerator = token_in_amount_u128
        .checked_mul(10_u128.pow((token_out_decimals + PRICE_DECIMALS) as u32))
        .ok_or(TokenUtilsErrorCode::MathOverflow)?;

    // Calculate: denominator = price * 10^token_in_decimals
    let denominator = price_u128
        .checked_mul(10_u128.pow(token_in_decimals as u32))
        .ok_or(TokenUtilsErrorCode::MathOverflow)?;

    Ok((numerator / denominator) as u64)
}

/// Formats a u64 number as a decimal string with 9 decimal places
///
/// This function treats the input as a fixed-point number with 9 decimal places,
/// where the last 9 digits represent the fractional part.
///
/// # Arguments
/// * `n` - The number to format, with the last 9 digits as the fractional part
///
/// # Returns
/// A string representation of the number with appropriate decimal formatting
///
/// # Examples
/// * `u64_to_dec9(1_500_000_000)` returns `"1.5"`
/// * `u64_to_dec9(123_456_789_012)` returns `"123.456789012"`
/// * `u64_to_dec9(1_000_000_000)` returns `"1"`
pub fn u64_to_dec9(n: u64) -> String {
    let int_part = n / 1_000_000_000;
    let frac_part = n % 1_000_000_000;

    if frac_part == 0 {
        return int_part.to_string();
    }
    let mut frac = format!("{:09}", frac_part);
    while frac.ends_with('0') {
        frac.pop();
    }

    format!("{}.{}", int_part, frac)
}

/// Result structure for fee calculation
pub struct CalculateFeeResult {
    /// The calculated fee amount in token_in units
    pub fee_amount: u64,
    /// The remaining token_in amount after fee deduction
    pub remaining_token_in_amount: u64,
}

/// Calculates fee amount and remaining amount after fee deduction
///
/// # Arguments
/// * `token_in_amount` - Total amount of token_in being processed
/// * `fee_basis_points` - Fee percentage in basis points (e.g., 500 = 5%)
///
/// # Returns
/// A `CalculateFeeResult` containing the fee amount and remaining amount
///
/// # Errors
/// * `MathOverflow` - If calculations exceed u128 limits
///
/// # Example
/// ```
/// // 5% fee on 1000 tokens = 50 fee, 950 remaining
/// let result = calculate_fees(1000, 500)?;
/// assert_eq!(result.fee_amount, 50);
/// assert_eq!(result.remaining_token_in_amount, 950);
/// ```
pub fn calculate_fees(token_in_amount: u64, fee_basis_points: u16) -> Result<CalculateFeeResult> {
    // Calculate fee amount in token_in tokens
    let fee_amount = (token_in_amount as u128)
        .checked_mul(fee_basis_points as u128)
        .ok_or(TokenUtilsErrorCode::MathOverflow)?
        .checked_div(MAX_BASIS_POINTS as u128)
        .ok_or(TokenUtilsErrorCode::MathOverflow)? as u64;

    // Amount after fee deduction for the main offer exchange
    let remaining_token_in_amount = token_in_amount
        .checked_sub(fee_amount)
        .ok_or(TokenUtilsErrorCode::MathOverflow)?;

    Ok(CalculateFeeResult {
        fee_amount,
        remaining_token_in_amount,
    })
}

/// Mint tokens with maximum supply validation
///
/// This function validates that minting the requested amount will not exceed
/// the configured maximum supply cap (if set). If max_supply is 0, no cap is enforced.
///
/// # Arguments
/// * `token_program` - The SPL Token program
/// * `mint` - The token mint to mint from
/// * `to_account` - Destination token account
/// * `authority` - The mint authority (must be a PDA with signing capability)
/// * `signer_seeds` - PDA seeds for program-signed minting
/// * `amount` - Amount of tokens to mint
/// * `max_supply` - Maximum supply cap (0 = no cap)
///
/// # Returns
/// * `Ok(())` - If minting completes successfully and doesn't exceed max supply
/// * `Err(TokenUtilsErrorCode::MaxSupplyExceeded)` - If minting would exceed the cap
/// * `Err(_)` - If token minting operation fails
///
/// # Security
/// - Validates current supply + amount <= max_supply (when max_supply > 0)
/// - Uses checked token instructions for decimal validation
/// - Prevents unbounded inflation when max supply is configured
pub fn mint_tokens<'info>(
    token_program: &Interface<'info, TokenInterface>,
    mint: &InterfaceAccount<'info, Mint>,
    to_account: &InterfaceAccount<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
    amount: u64,
    max_supply: u64,
) -> Result<()> {
    // Validate max supply if configured (0 = no cap)
    if max_supply > 0 {
        let current_supply = mint.supply;
        let new_supply = current_supply
            .checked_add(amount)
            .ok_or(TokenUtilsErrorCode::MathOverflow)?;

        require!(
            new_supply <= max_supply,
            TokenUtilsErrorCode::MaxSupplyExceeded
        );
    }

    // Perform the mint operation
    let mint_accounts = MintToChecked {
        mint: mint.to_account_info(),
        to: to_account.to_account_info(),
        authority: authority.to_account_info(),
    };

    let mint_ctx =
        CpiContext::new_with_signer(token_program.to_account_info(), mint_accounts, signer_seeds);

    token_interface::mint_to_checked(mint_ctx, amount, mint.decimals)
}

/// Burns tokens from a user account using user authority
///
/// # Arguments
/// * `token_program` - The SPL Token program
/// * `mint` - The token mint to burn from
/// * `from_account` - Source token account to burn from
/// * `authority` - The burn authority (the token account owner)
/// * `signer_seeds` - Optional PDA seeds for program-signed burning (None for user-signed)
/// * `amount` - Amount of tokens to burn
pub fn burn_tokens<'info>(
    token_program: &Interface<'info, TokenInterface>,
    mint: &InterfaceAccount<'info, Mint>,
    from_account: &InterfaceAccount<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<()> {
    let burn_accounts = BurnChecked {
        mint: mint.to_account_info(),
        from: from_account.to_account_info(),
        authority: authority.to_account_info(),
    };

    let cpi_context =
        CpiContext::new_with_signer(token_program.to_account_info(), burn_accounts, signer_seeds);

    token_interface::burn_checked(cpi_context, amount, mint.decimals)
}

/// Parameters for executing token exchange operations
///
/// This structure contains all the accounts and parameters needed to execute
/// a complete token exchange, handling both token_in payment and token_out distribution
/// with support for both mint/burn and transfer operations.
pub struct ExecTokenOpsParams<'a, 'info> {
    /// SPL Token program for token_in operations
    pub token_in_program: &'a Interface<'info, TokenInterface>,
    /// SPL Token program for token_out operations
    pub token_out_program: &'a Interface<'info, TokenInterface>,

    // Token in params
    /// Mint account for the input token
    pub token_in_mint: &'a InterfaceAccount<'info, Mint>,
    /// Amount of token_in to process
    pub token_in_amount: u64,
    /// Authority that can transfer from the source account
    pub token_in_authority: &'a AccountInfo<'info>,
    /// Optional PDA seeds for program-signed token_in transfers
    pub token_in_source_signer_seeds: Option<&'a [&'a [&'a [u8]]]>,
    /// PDA seeds for vault authority operations
    pub vault_authority_signer_seeds: Option<&'a [&'a [&'a [u8]]]>,
    /// Source account for token_in (user's account)
    pub token_in_source_account: &'a InterfaceAccount<'info, TokenAccount>,
    /// Destination account for token_in (boss's account)
    pub token_in_destination_account: &'a InterfaceAccount<'info, TokenAccount>,
    /// Vault account for burning token_in when program has mint authority
    pub token_in_burn_account: &'a InterfaceAccount<'info, TokenAccount>,
    /// Authority for burning tokens from the vault
    pub token_in_burn_authority: &'a AccountInfo<'info>,

    // Token out params
    /// Mint account for the output token
    pub token_out_mint: &'a InterfaceAccount<'info, Mint>,
    /// Amount of token_out to distribute
    pub token_out_amount: u64,
    /// Authority for token_out operations (vault authority)
    pub token_out_authority: &'a AccountInfo<'info>,
    /// Source account for token_out transfers (vault account)
    pub token_out_source_account: &'a InterfaceAccount<'info, TokenAccount>,
    /// Destination account for token_out (user's account)
    pub token_out_destination_account: &'a InterfaceAccount<'info, TokenAccount>,
    /// PDA for mint authority operations
    pub mint_authority_pda: &'a AccountInfo<'info>,
    /// Bump seed for mint authority PDA
    pub mint_authority_bump: &'a [u8],
    /// Maximum supply cap for token_out minting (0 = no cap)
    pub token_out_max_supply: u64,
}

/// Executes token operations for exchanging token_in for token_out
///
/// This function handles the complete token exchange process with intelligent routing
/// based on mint authority ownership. It supports both mint/burn and transfer operations
/// to provide maximum flexibility for different token configurations.
///
/// # Token In Processing
/// - If program has mint authority: transfers to vault → burns tokens (deflationary)
/// - If program lacks mint authority: transfers directly to boss/destination (standard transfer)
///
/// # Token Out Processing  
/// - If program has mint authority: mints directly to user (inflationary)
/// - If program lacks mint authority: transfers from vault to user (standard transfer)
///
/// # Arguments
/// * `params` - Complete parameter structure containing all required accounts and amounts
///
/// # Returns
/// * `Ok(())` - If all token operations complete successfully
/// * `Err(_)` - If any transfer, mint, or burn operation fails
///
/// # Security
/// - All operations use checked token instructions for decimal validation
/// - PDA seeds are used for program-signed operations
/// - Authority validation ensures only authorized transfers
pub fn execute_token_operations(params: ExecTokenOpsParams) -> Result<()> {
    // Step 1: User pays token_in
    let controls_token_in_mint =
        program_controls_mint(params.token_in_mint, params.mint_authority_pda);
    let token_in_destination = if controls_token_in_mint {
        // transfer to program owned PDA for burning
        params.token_in_burn_account
    } else {
        // transfer directly to boss/intermediary account (in permissionless flow)
        params.token_in_destination_account
    };

    transfer_tokens(
        params.token_in_mint,
        params.token_in_program,
        params.token_in_source_account,
        token_in_destination,
        params.token_in_authority,
        params.token_in_source_signer_seeds,
        params.token_in_amount,
    )?;

    if controls_token_in_mint {
        burn_tokens(
            params.token_in_program,
            params.token_in_mint,
            params.token_in_burn_account,
            params.token_in_burn_authority,
            params.vault_authority_signer_seeds.unwrap(),
            params.token_in_amount,
        )?;
    }

    // Step 2: Program distributes token_out
    if program_controls_mint(params.token_out_mint, params.mint_authority_pda) {
        let mint_authority_seeds = &[seeds::MINT_AUTHORITY, params.mint_authority_bump];
        let mint_authority_signer_seeds = &[mint_authority_seeds.as_slice()];

        mint_tokens(
            params.token_out_program,
            params.token_out_mint,
            params.token_out_destination_account,
            params.mint_authority_pda,
            mint_authority_signer_seeds,
            params.token_out_amount,
            params.token_out_max_supply,
        )?;
    } else {
        transfer_tokens(
            params.token_out_mint,
            params.token_out_program,
            params.token_out_source_account,
            params.token_out_destination_account,
            params.token_out_authority,
            params.vault_authority_signer_seeds,
            params.token_out_amount,
        )?;
    }

    Ok(())
}

/// Returns true iff `mint.mint_authority == Some(mint_authority_pda.key())`.
pub fn program_controls_mint<'info>(
    mint: &InterfaceAccount<'info, Mint>,
    mint_authority_pda: &AccountInfo<'info>,
) -> bool {
    matches!(mint.mint_authority, COption::Some(pk) if pk == mint_authority_pda.key())
}
