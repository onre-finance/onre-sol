use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::token::spl_token::instruction::AuthorityType;
use anchor_spl::token::{set_authority, SetAuthority};
use anchor_spl::token_interface::{Mint, TokenInterface};

/// Handles transferring mint authority from program PDA back to the boss account
///
/// This instruction serves as an emergency recovery mechanism allowing the boss to regain
/// direct control of mint authority. Common use cases include emergency recovery, temporary
/// manual token operations, program maintenance, or returning to pre-program authority setup.
///
/// # Security
/// - Only the current boss can initiate the transfer
/// - Program PDA must currently hold mint authority
/// - Uses program-derived signatures for authorization

/// Error codes for mint authority transfer to boss operations
#[error_code]
pub enum TransferMintAuthorityToBossErrorCode {
    /// The program PDA is not the current mint authority for the specified token
    #[msg("Program PDA must be the current mint authority")]
    ProgramNotMintAuthority,
}

/// Event emitted when mint authority is successfully transferred from program PDA to boss
///
/// Provides transparency for tracking mint authority changes and emergency recovery operations.
#[event]
pub struct MintAuthorityTransferredToBossEvent {
    /// The mint whose authority was transferred
    pub mint: Pubkey,
    /// The previous authority (program PDA)
    pub old_authority: Pubkey,
    /// The new authority (boss account)
    pub new_authority: Pubkey,
}

/// Account structure for transferring mint authority from program PDA to boss
///
/// This struct defines the accounts required for the boss to recover mint authority
/// from the program PDA. Serves as an emergency recovery mechanism for regaining
/// direct control over token minting operations.
#[derive(Accounts)]
pub struct TransferMintAuthorityToBoss<'info> {
    /// The boss account authorized to recover mint authority
    ///
    /// Must be the current boss stored in program state and sign the transaction
    /// to authorize the mint authority transfer.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state account containing boss validation
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = boss)]
    pub state: Account<'info, State>,

    /// The token mint whose authority will be transferred to the boss
    ///
    /// Must currently have the program PDA as its mint authority. The mint
    /// will be updated to have the boss as the new mint authority.
    #[account(
        mut,
        constraint = mint.mint_authority.unwrap() == mint_authority.key() @ TransferMintAuthorityToBossErrorCode::ProgramNotMintAuthority
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// Program-derived account that currently holds mint authority
    ///
    /// This PDA must be the current mint authority for the token. The program
    /// uses this PDA's signature to authorize transferring authority to the boss.
    /// CHECK: PDA derivation is validated by seeds constraint, authority is validated by mint constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        bump
    )]
    pub mint_authority: AccountInfo<'info>,

    /// SPL Token program for mint authority operations
    pub token_program: Interface<'info, TokenInterface>,
}

/// Transfers mint authority from program PDA back to the boss account
///
/// This instruction serves as an emergency recovery mechanism allowing the boss to
/// regain direct control of mint authority. The transfer uses the program's PDA
/// signature to prove the program is willingly relinquishing control.
///
/// After this operation, the boss will have direct mint authority and can mint tokens
/// outside the program's controlled minting mechanisms.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If authority transfer completes successfully
/// * `Err(TransferMintAuthorityToBossErrorCode::ProgramNotMintAuthority)` - If program PDA doesn't hold authority
/// * `Err(_)` - If SPL Token authority transfer fails
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Program PDA must currently hold mint authority
/// - Boss account must match the one stored in program state
///
/// # Events
/// * `MintAuthorityTransferredToBossEvent` - Emitted on successful authority transfer
pub fn transfer_mint_authority_to_boss(ctx: Context<TransferMintAuthorityToBoss>) -> Result<()> {
    // Construct PDA signer seeds for authorization
    let seeds = &[seeds::MINT_AUTHORITY, &[ctx.bumps.mint_authority]];
    let signer_seeds = &[seeds.as_slice()];

    // Transfer mint authority from program PDA back to boss using program signature
    set_authority(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            SetAuthority {
                current_authority: ctx.accounts.mint_authority.to_account_info(),
                account_or_mint: ctx.accounts.mint.to_account_info(),
            },
            signer_seeds,
        ),
        AuthorityType::MintTokens,
        Some(ctx.accounts.boss.key()),
    )?;

    // Emit event for transparency and off-chain tracking
    emit!(MintAuthorityTransferredToBossEvent {
        mint: ctx.accounts.mint.key(),
        old_authority: ctx.accounts.mint_authority.key(),
        new_authority: ctx.accounts.boss.key(),
    });

    Ok(())
}
