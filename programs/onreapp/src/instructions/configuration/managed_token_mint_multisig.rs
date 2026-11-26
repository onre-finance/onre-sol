//! Boss-only switching of a managed token's modular minting path between direct PDA
//! minting and a 1-of-N SPL multisig shared with other minters (e.g. a CCIP pool).
use crate::{
    constants::seeds::{MINT_AUTHORITY, STATE},
    model::{seeds::MANAGED_TOKEN, ManagedTokenConfig},
    state::State,
    utils::program_minter::{reclaim_mint_authority, validate_mint_multisig},
    OnreError,
};
use anchor_lang::{prelude::*, solana_program::program_option::COption};
use anchor_spl::token_interface::{Mint, TokenInterface};

#[event]
pub struct ManagedTokenMintMultisigSetEvent {
    pub boss: Pubkey,
    pub managed_mint: Pubkey,
    pub previous: Pubkey,
    /// Default when direct PDA minting was restored.
    pub mint_multisig: Pubkey,
    pub timestamp: i64,
}

/// Shared accounts for enabling and disabling multisig minting.
#[derive(Accounts)]
pub struct SetManagedTokenMintMultisig<'info> {
    #[account(
        mut,
        seeds = [MANAGED_TOKEN, managed_mint.key().as_ref()],
        bump = managed_token.bump,
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    #[account(mut, mint::token_program = token_program)]
    pub managed_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: Canonical program mint-authority PDA; validated by seeds.
    #[account(seeds = [MINT_AUTHORITY], bump)]
    pub mint_authority: UncheckedAccount<'info>,

    /// CHECK: SPL multisig validated in the handler: enable checks its contents and
    /// mint authority; disable requires the currently configured multisig.
    pub mint_multisig: UncheckedAccount<'info>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

/// Selects `mint_multisig` for modular minting. The boss must already have made it the
/// mint authority (e.g. `transfer_mint_authority_to_boss`, then SPL `set_authority` to
/// the multisig, then this instruction, all in one transaction). The multisig must be
/// an initialized 1-of-N multisig of the mint's token program that includes the
/// program's mint-authority PDA. Legacy instructions remain direct-only, so ONyc
/// requires the one-way BUFFER switch first: before it, legacy instructions would
/// stop minting and burning ONyc and fall back to vault transfers.
pub fn enable_managed_token_mint_multisig(ctx: Context<SetManagedTokenMintMultisig>) -> Result<()> {
    let a = ctx.accounts;
    let multisig = a.mint_multisig.key();
    require_keys_neq!(a.managed_token.mint_multisig, multisig, OnreError::NoChange);
    require!(
        a.managed_mint.key() != a.state.onyc_mint || !a.state.onyc_legacy_buffer_active(),
        OnreError::MintMultisigBeforeBufferSwitch
    );
    validate_mint_multisig(
        &a.mint_multisig.to_account_info(),
        &a.token_program.key(),
        &a.mint_authority.key(),
    )?;
    require!(
        a.managed_mint.mint_authority == COption::Some(multisig),
        OnreError::ProgramNotMintAuthority
    );
    set(&mut a.managed_token, multisig, a.boss.key())
}

/// Restores direct PDA minting. If the configured multisig still holds mint authority,
/// the program reclaims it to its PDA by signing as the multisig member; otherwise the
/// PDA must already be the mint authority.
pub fn disable_managed_token_mint_multisig(
    ctx: Context<SetManagedTokenMintMultisig>,
) -> Result<()> {
    let a = ctx.accounts;
    let configured = a.managed_token.mint_multisig().ok_or(OnreError::NoChange)?;
    require_keys_eq!(
        a.mint_multisig.key(),
        configured,
        OnreError::InvalidMintMultisig
    );
    if a.managed_mint.mint_authority == COption::Some(configured) {
        reclaim_mint_authority(
            &a.token_program,
            &a.managed_mint,
            &a.mint_multisig.to_account_info(),
            &a.mint_authority.to_account_info(),
            ctx.bumps.mint_authority,
        )?;
    } else {
        require!(
            a.managed_mint.mint_authority == COption::Some(a.mint_authority.key()),
            OnreError::ProgramNotMintAuthority
        );
    }
    set(&mut a.managed_token, Pubkey::default(), a.boss.key())
}

fn set(managed_token: &mut ManagedTokenConfig, mint_multisig: Pubkey, boss: Pubkey) -> Result<()> {
    let previous = managed_token.mint_multisig;
    managed_token.mint_multisig = mint_multisig;
    emit!(ManagedTokenMintMultisigSetEvent {
        boss,
        managed_mint: managed_token.managed_mint,
        previous,
        mint_multisig,
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
