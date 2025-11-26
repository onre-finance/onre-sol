use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022::instruction::AuthorityType;
use anchor_spl::token_interface::{set_authority, SetAuthority};
use anchor_spl::token_interface::{Mint, TokenInterface};

/// Handles transferring mint authority from the boss account to a program PDA
///
/// This enables burn/mint token architecture allowing the program to mint tokens directly
/// instead of transferring from pre-minted vaults. Essential for controlled token supply
/// management and programmatic minting operations.
///
/// # Security
/// - Only the current boss can transfer mint authority
/// - Boss must be the current mint authority for the token
/// - Authority can be recovered using `transfer_mint_authority_to_boss`

/// Error codes for mint authority transfer to program operations
#[error_code]
pub enum TransferMintAuthorityToProgramErrorCode {
    /// The boss is not the current mint authority for the specified token
    #[msg("Boss must be the current mint authority")]
    BossNotMintAuthority,
}

/// Event emitted when mint authority is successfully transferred from boss to program PDA
///
/// Provides transparency for tracking mint authority changes and enabling programmatic control.
#[event]
pub struct MintAuthorityTransferredToProgramEvent {
    /// The mint whose authority was transferred
    pub mint: Pubkey,
    /// The previous authority (boss account)
    pub old_authority: Pubkey,
    /// The new authority (program PDA)
    pub new_authority: Pubkey,
}

/// Account structure for transferring mint authority from boss to program PDA
///
/// This struct defines the accounts required for the boss to transfer mint authority
/// to the program PDA, enabling programmatic token minting and burn/mint architecture.
#[derive(Accounts)]
pub struct TransferMintAuthorityToProgram<'info> {
    /// The boss account authorized to transfer mint authority
    ///
    /// Must be the current boss stored in program state and currently hold
    /// mint authority for the specified token.
    pub boss: Signer<'info>,

    /// Program state account containing boss validation
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = boss)]
    pub state: Account<'info, State>,

    /// The token mint whose authority will be transferred to the program
    ///
    /// Must currently have the boss as its mint authority. After the transfer,
    /// the program PDA will be able to mint tokens programmatically.
    #[account(
        mut,
        constraint = mint.mint_authority.unwrap() == boss.key() @ TransferMintAuthorityToProgramErrorCode::BossNotMintAuthority
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// Program-derived account that will become the new mint authority
    ///
    /// This PDA will receive mint authority and enable the program to mint
    /// tokens directly for controlled supply management operations.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        bump
    )]
    pub mint_authority: AccountInfo<'info>,

    /// SPL Token program for mint authority operations
    pub token_program: Interface<'info, TokenInterface>,
}

/// Transfers mint authority from the boss account to a program PDA
///
/// This instruction enables burn/mint functionality by giving the program direct
/// control over token minting. After this transfer, the program can mint tokens
/// programmatically using the PDA's authority for controlled supply management.
///
/// The boss retains the ability to recover mint authority using the
/// `transfer_mint_authority_to_boss` instruction if needed.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If authority transfer completes successfully
/// * `Err(TransferMintAuthorityToProgramErrorCode::BossNotMintAuthority)` - If boss doesn't hold authority
/// * `Err(_)` - If SPL Token authority transfer fails
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss must currently hold mint authority
/// - Boss account must match the one stored in program state
///
/// # Events
/// * `MintAuthorityTransferredToProgramEvent` - Emitted on successful authority transfer
pub fn transfer_mint_authority_to_program(
    ctx: Context<TransferMintAuthorityToProgram>,
) -> Result<()> {
    let mint_authority = ctx.accounts.mint_authority.key();

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
        Some(mint_authority),
    )?;

    // Emit event for transparency and off-chain tracking
    emit!(MintAuthorityTransferredToProgramEvent {
        mint: ctx.accounts.mint.key(),
        old_authority: ctx.accounts.boss.key(),
        new_authority: mint_authority,
    });

    Ok(())
}
