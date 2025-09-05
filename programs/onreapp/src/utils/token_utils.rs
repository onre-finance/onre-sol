use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

pub const MAX_BASIS_POINTS: u64 = 10000;

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
    token_program: &Program<'info, Token>,
    from_account: &Account<'info, TokenAccount>,
    to_account: &Account<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    signer_seeds: Option<&[&[&[u8]]]>,
    amount: u64,
) -> Result<()> {
    let transfer_accounts = Transfer {
        from: from_account.to_account_info(),
        to: to_account.to_account_info(),
        authority: authority.to_account_info(),
    };

    let transfer_ctx = match signer_seeds {
        Some(seeds) => {
            CpiContext::new_with_signer(token_program.to_account_info(), transfer_accounts, seeds)
        }
        None => CpiContext::new(token_program.to_account_info(), transfer_accounts),
    };

    token::transfer(transfer_ctx, amount)
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
        .checked_mul(10_u128.pow((token_out_decimals + 9) as u32))
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
pub fn calculate_fees(token_in_amount: u64, fee_basis_points: u64) -> Result<CalculateFeeResult> {
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
    token_program: &Program<'info, Token>,
    mint: &Account<'info, Mint>,
    to_account: &Account<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<()> {
    let mint_accounts = MintTo {
        mint: mint.to_account_info(),
        to: to_account.to_account_info(),
        authority: authority.to_account_info(),
    };

    let mint_ctx =
        CpiContext::new_with_signer(token_program.to_account_info(), mint_accounts, signer_seeds);

    token::mint_to(mint_ctx, amount)
}

/// Enhanced token distribution function that mints directly or falls back to vault transfer
///
/// This function implements the core logic for buy offer token distribution:
/// 1. Checks if the program has mint authority for the token
/// 2. If yes: Mints tokens directly to user (more efficient)
/// 3. If no: Transfers tokens from vault to user (fallback)
///
/// # Arguments
/// * `token_out_mint` - The mint for the output token
/// * `mint_authority_pda` - Optional mint authority PDA (if program has authority)
/// * `vault_authority` - Vault authority PDA (for transfer fallback)
/// * `vault_token_out_account` - Optional vault token account (for transfer fallback)
/// * `user_token_out_account` - User's destination token account
/// * `token_program` - SPL Token program
/// * `vault_authority_bump` - Bump seed for vault authority
/// * `mint_authority_bump` - Optional bump seed for mint authority
/// * `token_out_amount` - Amount of tokens to distribute
///
/// # Returns
/// * `Ok(())` if tokens were successfully distributed (either minted or transferred)
/// * `Err(_)` if both mint and transfer attempts fail
pub fn mint_or_transfer_tokens<'info>(
    token_out_mint: &Account<'info, Mint>,
    mint_authority_pda: Option<&AccountInfo<'info>>,
    vault_authority: &AccountInfo<'info>,
    vault_token_out_account: Option<&Account<'info, TokenAccount>>,
    user_token_out_account: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    vault_authority_bump: u8,
    mint_authority_bump: Option<u8>,
    token_out_amount: u64,
) -> Result<()> {
    use crate::constants::seeds;

    // Check if program has mint authority and all required accounts are present
    if let (Some(mint_authority), Some(bump)) = (mint_authority_pda, mint_authority_bump) {
        // Verify that the program actually holds the mint authority
        let expected_authority = mint_authority.key();
        match token_out_mint.mint_authority {
            anchor_lang::solana_program::program_option::COption::Some(current_authority) => {
                if current_authority == expected_authority {
                    // Program has mint authority - mint directly to user
                    let mint_key = token_out_mint.key();
                    let authority_seeds = &[seeds::MINT_AUTHORITY, mint_key.as_ref(), &[bump]];
                    let signer_seeds = &[authority_seeds.as_slice()];

                    return mint_tokens(
                        token_program,
                        token_out_mint,
                        user_token_out_account,
                        mint_authority,
                        signer_seeds,
                        token_out_amount,
                    );
                }
            }
            _ => {} // No mint authority or different authority - fall through to transfer
        }
    }

    // Fallback: Transfer from vault to user
    if let Some(vault_account) = vault_token_out_account {
        let vault_authority_seeds = &[seeds::BUY_OFFER_VAULT_AUTHORITY, &[vault_authority_bump]];
        let signer_seeds = &[vault_authority_seeds.as_slice()];

        return transfer_tokens(
            token_program,
            vault_account,
            user_token_out_account,
            vault_authority,
            Some(signer_seeds),
            token_out_amount,
        );
    }

    // This should never happen if account validation is correct
    err!(TokenUtilsErrorCode::MathOverflow) // Reusing existing error for simplicity
}
