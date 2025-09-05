use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::token::spl_token::instruction::AuthorityType;
use anchor_spl::token::{set_authority, Mint, SetAuthority, Token};

/// This module handles transferring mint authority from a program-derived PDA back to the boss account.
///
/// This instruction serves as an emergency recovery mechanism and allows the boss to regain control
/// of mint authority if needed. Common use cases include:
/// - Emergency recovery in case of program issues
/// - Temporary need to mint tokens outside the program
/// - Program maintenance or upgrades requiring manual token operations
/// - Returning to the original pre-program mint authority setup
///
/// # Security Considerations
/// - Only the current boss can initiate the transfer back
/// - The program PDA must be the current mint authority
/// - Uses program-derived signatures to authorize the transfer
/// - Each token has its own independent mint authority PDA

/// Error codes specific to transferring mint authority back to boss
#[error_code]
pub enum TransferMintAuthorityToBossErrorCode {
    /// Error when the program PDA does not currently hold mint authority
    #[msg("Program PDA must be the current mint authority")]
    ProgramNotMintAuthority,
}

/// Event emitted when mint authority is successfully transferred from program PDA back to boss
#[event]
pub struct MintAuthorityTransferredToBossEvent {
    /// The mint whose authority was transferred
    pub mint: Pubkey,
    /// The previous authority (program PDA)
    pub old_authority: Pubkey,
    /// The new authority (boss)
    pub new_authority: Pubkey,
}

/// Account structure for transferring mint authority from program PDA back to boss
///
/// This instruction allows the boss to recover mint authority from the program PDA.
/// It's designed as an emergency recovery mechanism and for situations where manual
/// control of minting is temporarily required.
///
/// # Account Requirements
/// - `boss` must be the current boss as defined in program state
/// - `mint` must currently have the program PDA as its mint authority
/// - `mint_authority_pda` must be the derived PDA that currently holds authority
#[derive(Accounts)]
pub struct TransferMintAuthorityToBoss<'info> {
    /// The current boss account, must sign the transaction
    /// Must match the boss stored in the program state
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state containing the current boss public key
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The token mint whose authority will be transferred back to boss
    /// Must currently have the program PDA as its mint authority
    #[account(
        mut,
        constraint = mint.mint_authority.unwrap() == mint_authority_pda.key() @ TransferMintAuthorityToBossErrorCode::ProgramNotMintAuthority
    )]
    pub mint: Account<'info, Mint>,

    /// Program-derived account that currently holds mint authority
    /// Must be derived from [MINT_AUTHORITY, mint_pubkey] and currently be the mint authority
    /// CHECK: PDA derivation is validated by seeds constraint, authority is validated by mint constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY, mint.key().as_ref()],
        bump
    )]
    pub mint_authority_pda: UncheckedAccount<'info>,

    /// SPL Token program for mint authority operations
    pub token_program: Program<'info, Token>,
}

/// Transfers mint authority from a program-derived PDA back to the boss account
///
/// This instruction serves as an emergency recovery mechanism and allows the boss to
/// regain control of mint authority. The transfer is authorized using the program's
/// PDA signature, proving that the program is willingly relinquishing control.
///
/// # Process
/// 1. Validates that the program PDA currently holds mint authority
/// 2. Constructs PDA signer seeds for authorization
/// 3. Uses SPL Token's `set_authority` with PDA signature to transfer to boss
/// 4. Emits an event for transparency and off-chain tracking
///
/// # Arguments
/// * `ctx` - The instruction context containing all required accounts
///
/// # Returns
/// * `Ok(())` if the transfer succeeds
/// * `Err` if validation fails or the SPL Token instruction fails
///
/// # Events
/// Emits `MintAuthorityTransferredToBossEvent` on success
pub fn transfer_mint_authority_to_boss(ctx: Context<TransferMintAuthorityToBoss>) -> Result<()> {
    let mint_key = ctx.accounts.mint.key();

    // Construct PDA signer seeds for authorization
    let seeds = &[
        seeds::MINT_AUTHORITY,
        mint_key.as_ref(),
        &[ctx.bumps.mint_authority_pda],
    ];
    let signer_seeds = &[seeds.as_slice()];

    // Transfer mint authority from program PDA back to boss using program signature
    set_authority(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            SetAuthority {
                current_authority: ctx.accounts.mint_authority_pda.to_account_info(),
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
        old_authority: ctx.accounts.mint_authority_pda.key(),
        new_authority: ctx.accounts.boss.key(),
    });

    Ok(())
}
