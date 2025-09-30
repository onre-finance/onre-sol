use crate::constants::seeds;
use crate::state::{MintAuthority, State};
use crate::utils::token_utils::mint_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Event emitted when ONyc tokens are minted to the boss
#[event]
pub struct OnycTokensMinted {
    /// The ONyc mint from which tokens were minted
    pub onyc_mint: Pubkey,
    /// The boss account that received the minted tokens
    pub boss: Pubkey,
    /// The amount of tokens minted
    pub amount: u64,
}

/// Error codes specific to mint_to instruction
#[error_code]
pub enum MintToErrorCode {
    /// Error when the mint provided doesn't match the ONyc mint in state
    #[msg("Provided mint does not match the ONyc mint in state")]
    InvalidOnycMint,
    /// Error when the program doesn't have mint authority
    #[msg("Program does not have mint authority for this token")]
    NoMintAuthority,
}

#[derive(Accounts)]
pub struct MintTo<'info> {
    /// The program state account, containing the boss and onyc_mint
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = boss, has_one = onyc_mint)]
    pub state: Account<'info, State>,

    /// The boss who is authorized to perform the minting operation
    #[account(mut)]
    pub boss: Signer<'info>,

    /// The ONyc token mint - must match the one stored in state
    #[account(mut)]
    pub onyc_mint: InterfaceAccount<'info, Mint>,

    /// The boss's ONyc token account
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = onyc_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_program
    )]
    pub boss_onyc_account: InterfaceAccount<'info, TokenAccount>,

    /// Program-derived account that serves as the mint authority
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        bump = mint_authority.bump,
        constraint = onyc_mint.mint_authority.unwrap() == mint_authority.key() @ MintToErrorCode::NoMintAuthority
    )]
    pub mint_authority: Account<'info, MintAuthority>,

    /// SPL Token program for minting operations
    pub token_program: Interface<'info, TokenInterface>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program required for account creation
    pub system_program: Program<'info, System>,
}

/// Mints ONyc tokens to the boss's account
///
/// This instruction allows the boss to mint new ONyc tokens directly to their own account.
/// The program must have mint authority for the ONyc token (transferred via transfer_mint_authority_to_program).
///
/// # Process
/// 1. Validates that the provided mint matches the ONyc mint stored in state
/// 2. Validates that the program has mint authority for the ONyc token
/// 3. Creates the boss's ONyc token account if it doesn't exist
/// 4. Mints the specified amount of ONyc tokens to the boss's account
/// 5. Emits an event for transparency and off-chain tracking
///
/// # Arguments
/// * `ctx` - The instruction context containing all required accounts
/// * `amount` - The amount of ONyc tokens to mint
///
/// # Returns
/// * `Ok(())` if the minting succeeds
/// * `Err` if validation fails or the minting operation fails
///
/// # Events
/// Emits `OnycTokensMinted` on success
pub fn mint_to(ctx: Context<MintTo>, amount: u64) -> Result<()> {
    let mint_authority_seeds = &[seeds::MINT_AUTHORITY, &[ctx.accounts.mint_authority.bump]];
    let mint_authority_signer_seeds = &[mint_authority_seeds.as_slice()];

    // Mint tokens to the boss's ONyc account
    mint_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.onyc_mint,
        &ctx.accounts.boss_onyc_account,
        &ctx.accounts.mint_authority.to_account_info(),
        mint_authority_signer_seeds,
        amount,
    )?;

    msg!("Minted {} ONyc tokens to boss account", amount);

    // Emit event for transparency and off-chain tracking
    emit!(OnycTokensMinted {
        onyc_mint: ctx.accounts.onyc_mint.key(),
        boss: ctx.accounts.boss.key(),
        amount,
    });

    Ok(())
}
