use crate::constants::seeds;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token_interface;
use anchor_spl::token_interface::{
    BurnChecked, Mint, MintToChecked, TokenAccount, TokenInterface, TransferChecked,
};

pub const MAX_BASIS_POINTS: u32 = 10000;
pub const PRICE_DECIMALS: u8 = 9;

#[error_code]
pub enum TokenUtilsErrorCode {
    #[msg("Math overflow")]
    MathOverflow,
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

/// Formats an u64 number as a decimal string where the last 9 digits are the fraction
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

pub struct CalculateFeeResult {
    pub fee_amount: u64,
    pub remaining_token_in_amount: u64,
}
pub fn calculate_fees(token_in_amount: u64, fee_basis_points: u32) -> Result<CalculateFeeResult> {
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

/// Mint tokens directly to a destination account using program authority
///
/// # Arguments
/// * `token_program` - The SPL Token program
/// * `mint` - The token mint to mint from
/// * `to_account` - Destination token account
/// * `authority` - The mint authority (must be a PDA with signing capability)
/// * `signer_seeds` - PDA seeds for program-signed minting
/// * `amount` - Amount of tokens to mint
pub fn mint_tokens<'info>(
    token_program: &Interface<'info, TokenInterface>,
    mint: &InterfaceAccount<'info, Mint>,
    to_account: &InterfaceAccount<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<()> {
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

pub struct ExecTokenOpsParams<'a, 'info> {
    pub token_in_program: &'a Interface<'info, TokenInterface>,
    pub token_out_program: &'a Interface<'info, TokenInterface>,
    // Token in params
    pub token_in_mint: &'a InterfaceAccount<'info, Mint>,
    pub token_in_amount: u64,
    pub token_in_authority: &'a AccountInfo<'info>,
    pub token_in_source_signer_seeds: Option<&'a [&'a [&'a [u8]]]>,
    pub vault_authority_signer_seeds: Option<&'a [&'a [&'a [u8]]]>,
    pub token_in_source_account: &'a InterfaceAccount<'info, TokenAccount>,
    pub token_in_destination_account: &'a InterfaceAccount<'info, TokenAccount>,
    pub token_in_burn_account: &'a InterfaceAccount<'info, TokenAccount>,
    pub token_in_burn_authority: &'a AccountInfo<'info>,
    // Token out params
    pub token_out_mint: &'a InterfaceAccount<'info, Mint>,
    pub token_out_amount: u64,
    pub token_out_authority: &'a UncheckedAccount<'info>,
    pub token_out_source_account: &'a InterfaceAccount<'info, TokenAccount>,
    pub token_out_destination_account: &'a InterfaceAccount<'info, TokenAccount>,
    pub mint_authority_pda: &'a AccountInfo<'info>,
    pub mint_authority_bump: &'a [u8],
}

/// Executes token operations for exchanging token_in for a single token_out.
///
/// If the program has mint authority for token_in, it transfers tokens from user to program vault
/// and burns them. Otherwise, it transfers directly to the boss.
///
/// If the program has mint authority for token_out, it mints directly to the user. Otherwise,
/// it transfers tokens from the vault to the user.
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
