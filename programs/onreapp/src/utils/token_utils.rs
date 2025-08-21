use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

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
/// let authority_seeds = &[b"vault_authority", &[bump]];
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