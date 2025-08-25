use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

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
/// 
/// # Examples
/// ```rust
/// // Regular user-signed transfer
/// transfer_tokens(
///     &ctx.accounts.token_program,
///     &ctx.accounts.user_token_account,
///     &ctx.accounts.recipient_account,
///     &ctx.accounts.user,
///     None,
///     amount,
/// )?;
/// 
/// // PDA-signed transfer
/// let authority_seeds = &[seeds::VAULT_AUTHORITY, &[bump]];
/// let signer_seeds = &[authority_seeds.as_slice()];
/// transfer_tokens(
///     &ctx.accounts.token_program,
///     &ctx.accounts.vault_account,
///     &ctx.accounts.user_account,
///     &ctx.accounts.vault_authority,
///     Some(signer_seeds),
///     amount,
/// )?;
/// ```
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
        Some(seeds) => CpiContext::new_with_signer(
            token_program.to_account_info(),
            transfer_accounts,
            seeds,
        ),
        None => CpiContext::new(
            token_program.to_account_info(),
            transfer_accounts,
        ),
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