use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::token::spl_token::instruction::AuthorityType;
use anchor_spl::token::{set_authority, Mint, SetAuthority, Token};

/// This module handles transferring mint authority from the boss account to a program-derived PDA.
///
/// The mint authority transfer is part of the burn/mint token architecture that allows the program
/// to mint tokens directly instead of transferring from pre-minted vaults.
///
/// # Security Considerations
/// - Only the current boss can transfer mint authority
/// - The boss must be the current mint authority for the token
/// - Each token gets its own unique mint authority PDA
/// - Authority can be recovered using `transfer_mint_authority_to_boss`

/// Error codes specific to transferring mint authority to program
#[error_code]
pub enum TransferMintAuthorityToProgramErrorCode {
    /// Error when the boss does not currently hold mint authority for the token
    #[msg("Boss must be the current mint authority")]
    BossNotMintAuthority,
}

/// Event emitted when mint authority is successfully transferred from boss to program PDA
#[event]
pub struct MintAuthorityTransferredToProgramEvent {
    /// The mint whose authority was transferred
    pub mint: Pubkey,
    /// The previous authority (boss)
    pub old_authority: Pubkey,
    /// The new authority (program PDA)
    pub new_authority: Pubkey,
}

/// Account structure for transferring mint authority from boss to program PDA
///
/// This instruction allows the boss to transfer mint authority for a specific token
/// to a program-derived account. The PDA is derived deterministically from the mint
/// address, ensuring each token has its own unique mint authority PDA.
///
/// # Account Requirements
/// - `boss` must be the current boss and current mint authority
/// - `mint` must have the boss as its current mint authority
/// - `mint_authority_pda` is derived from [MINT_AUTHORITY, mint_pubkey]
#[derive(Accounts)]
pub struct TransferMintAuthorityToProgram<'info> {
    /// The current boss account, must sign the transaction
    /// Must match the boss stored in the program state
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state containing the current boss public key
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The token mint whose authority will be transferred
    /// Must currently have the boss as its mint authority
    #[account(
        mut,
        constraint = mint.mint_authority.unwrap() == boss.key() @ TransferMintAuthorityToProgramErrorCode::BossNotMintAuthority
    )]
    pub mint: Account<'info, Mint>,

    /// Program-derived account that will become the new mint authority
    /// Derived from [MINT_AUTHORITY, mint_pubkey] to ensure uniqueness per token
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY, mint.key().as_ref()],
        bump
    )]
    pub mint_authority_pda: UncheckedAccount<'info>,

    /// SPL Token program for mint authority operations
    pub token_program: Program<'info, Token>,
}

/// Transfers mint authority from the boss account to a program-derived PDA
///
/// This instruction is the first step in enabling burn/mint functionality for the program.
/// After calling this instruction, the program will be able to mint tokens using the
/// derived PDA as the mint authority.
///
/// # Process
/// 1. Validates that the boss is the current mint authority
/// 2. Uses SPL Token's `set_authority` to transfer mint authority to the PDA
/// 3. Emits an event for transparency and off-chain tracking
///
/// # Arguments
/// * `ctx` - The instruction context containing all required accounts
///
/// # Returns
/// * `Ok(())` if the transfer succeeds
/// * `Err` if validation fails or the SPL Token instruction fails
///
/// # Events
/// Emits `MintAuthorityTransferredToProgramEvent` on success
pub fn transfer_mint_authority_to_program(
    ctx: Context<TransferMintAuthorityToProgram>,
) -> Result<()> {
    let mint_authority_pda = ctx.accounts.mint_authority_pda.key();

    // Transfer mint authority from boss to program PDA using SPL Token
    set_authority(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            SetAuthority {
                current_authority: ctx.accounts.boss.to_account_info(),
                account_or_mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        AuthorityType::MintTokens,
        Some(mint_authority_pda),
    )?;

    // Emit event for transparency and off-chain tracking
    emit!(MintAuthorityTransferredToProgramEvent {
        mint: ctx.accounts.mint.key(),
        old_authority: ctx.accounts.boss.key(),
        new_authority: mint_authority_pda,
    });

    Ok(())
}
