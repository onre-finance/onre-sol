use crate::constants::seeds;
use crate::state::{MintAuthority, State};
use crate::utils::token_utils::mint_tokens;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Event emitted when ONyc tokens are successfully minted to the boss account
///
/// Provides transparency for tracking token minting operations performed by the boss.
#[event]
pub struct OnycTokensMintedEvent {
    /// The ONyc mint from which tokens were minted
    pub onyc_mint: Pubkey,
    /// The boss account that received the newly minted tokens
    pub boss: Pubkey,
    /// The amount of tokens minted in base units
    pub amount: u64,
}

/// Error codes for mint_to instruction operations
#[error_code]
pub enum MintToErrorCode {
    /// The provided mint doesn't match the ONyc mint stored in program state
    #[msg("Provided mint does not match the ONyc mint in state")]
    InvalidOnycMint,
    /// The program doesn't have mint authority for the specified token
    #[msg("Program does not have mint authority for this token")]
    NoMintAuthority,
}

/// Account structure for minting ONyc tokens to the boss
///
/// This struct defines the accounts required for the boss to mint new ONyc tokens
/// to their own account. Requires program mint authority and boss authorization.
#[derive(Accounts)]
pub struct MintTo<'info> {
    /// The program state account containing boss and ONyc mint validation
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = boss, has_one = onyc_mint)]
    pub state: Account<'info, State>,

    /// The boss authorized to perform minting operations
    ///
    /// Must be the boss stored in the program state and pay for any
    /// account creation if the token account doesn't exist.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// The ONyc token mint account for minting new tokens
    ///
    /// Must match the ONyc mint stored in program state and be mutable
    /// to allow supply updates during minting.
    #[account(mut)]
    pub onyc_mint: InterfaceAccount<'info, Mint>,

    /// The boss's ONyc token account to receive minted tokens
    ///
    /// If the account doesn't exist, it will be created automatically
    /// as an Associated Token Account with the boss as the authority.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = onyc_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_program
    )]
    pub boss_onyc_account: InterfaceAccount<'info, TokenAccount>,

    /// Program-derived account that serves as the mint authority
    ///
    /// This PDA must be the current mint authority for the ONyc token.
    /// Validated to ensure the program has permission to mint tokens.
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

    /// System program required for account creation and rent payment
    pub system_program: Program<'info, System>,
}

/// Mints new ONyc tokens directly to the boss's account
///
/// This instruction allows the boss to create new ONyc tokens and add them to their
/// own token account. The operation requires the program to have mint authority for
/// the ONyc token, which must be transferred via `transfer_mint_authority_to_program`.
///
/// The boss's token account is created automatically if it doesn't exist. The minting
/// operation increases the total supply of ONyc tokens and emits an event for tracking.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `amount` - The amount of ONyc tokens to mint in base units
///
/// # Returns
/// * `Ok(())` - If minting completes successfully
/// * `Err(MintToErrorCode::NoMintAuthority)` - If program lacks mint authority
/// * `Err(_)` - If token minting operation fails
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Program must have mint authority for the ONyc token
/// - Boss account must match the one stored in program state
///
/// # Events
/// * `OnycTokensMinted` - Emitted on successful minting with details
pub fn mint_to(ctx: Context<MintTo>, amount: u64) -> Result<()> {
    let mint_authority_seeds = &[seeds::MINT_AUTHORITY, &[ctx.accounts.mint_authority.bump]];
    let mint_authority_signer_seeds = &[mint_authority_seeds.as_slice()];

    // Mint tokens to the boss's ONyc account with max supply validation
    mint_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.onyc_mint,
        &ctx.accounts.boss_onyc_account,
        &ctx.accounts.mint_authority.to_account_info(),
        mint_authority_signer_seeds,
        amount,
        ctx.accounts.state.max_supply,
    )?;

    msg!("Minted {} ONyc tokens to boss account", amount);

    // Emit event for transparency and off-chain tracking
    emit!(OnycTokensMintedEvent {
        onyc_mint: ctx.accounts.onyc_mint.key(),
        boss: ctx.accounts.boss.key(),
        amount,
    });

    Ok(())
}
