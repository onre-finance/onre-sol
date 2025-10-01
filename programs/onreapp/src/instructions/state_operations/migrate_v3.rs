use crate::constants::seeds;
use crate::state::{PermissionlessAuthority, State};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::Discriminator;

// Old size of State (accounting for the 8-byte discriminator).
// Old State only had: boss (32 bytes)
const OLD_STATE_SPACE: usize = 8 + 32;

// New size (with InitSpace the const excludes the 8-byte discriminator).
// New State has: boss (32 bytes) + is_killed (1 byte) + onyc_mint (32 bytes) + admins array (20 * 32 = 640 bytes)
const NEW_STATE_SPACE: usize = 8 + <State as Space>::INIT_SPACE;

// Old size of PermissionlessAuthority (accounting for the 8-byte discriminator)
const OLD_PERMISSIONLESS_SPACE: usize = 8 + 4 + 50;

// New size (with InitSpace the const excludes the 8-byte discriminator).
const NEW_PERMISSIONLESS_SPACE: usize = 8 + <PermissionlessAuthority as Space>::INIT_SPACE;

#[error_code]
pub enum MigrationError {
    #[msg("Invalid State account data")]
    InvalidStateData,
    #[msg("Invalid PermissionlessAuthority account data")]
    InvalidPermissionlessData,
    #[msg("Boss pubkey mismatch")]
    BossMismatch,
}

#[derive(Accounts)]
pub struct MigrateV3<'info> {
    /// CHECK: handled manually to avoid deserializing the new struct over old data
    #[account(mut)]
    pub state: AccountInfo<'info>,

    /// CHECK: handled manually to avoid deserializing the new struct over old data
    #[account(mut)]
    pub permissionless_authority: AccountInfo<'info>,

    /// The boss who is authorized to perform the migration
    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn migrate_v3(ctx: Context<MigrateV3>) -> Result<()> {
    migrate_state(&ctx)?; // Signer == boss is validated inside migrate_state
    migrate_permissionless(&ctx)?;

    Ok(())
}

fn migrate_state(ctx: &Context<MigrateV3>) -> Result<()> {
    let stored_boss = {
        let data = ctx.accounts.state.try_borrow_data()?;

        require!(
            data.len() == OLD_STATE_SPACE || data.len() == NEW_STATE_SPACE,
            MigrationError::InvalidStateData
        );
        require!(
            data.starts_with(&State::DISCRIMINATOR),
            MigrationError::InvalidStateData
        );

        // boss is stored at bytes [8..40] in the old layout
        Pubkey::new_from_array(data[8..8 + 32].try_into().unwrap())
    }; // data borrow is dropped here

    require_keys_eq!(
        stored_boss,
        ctx.accounts.boss.key(),
        MigrationError::BossMismatch
    );

    top_up_lamports(
        ctx.accounts.state.to_account_info(),
        ctx.accounts.boss.to_account_info(),
        NEW_STATE_SPACE,
        ctx.accounts.system_program.to_account_info(),
    )?;

    let (pda, bump) = Pubkey::find_program_address(&[seeds::STATE], ctx.program_id);
    require_keys_eq!(
        pda,
        ctx.accounts.state.key(),
        MigrationError::InvalidStateData
    );

    // Realloc
    ctx.accounts.state.realloc(NEW_STATE_SPACE, true)?;

    // Now get a fresh mutable borrow after realloc
    let mut data = ctx.accounts.state.try_borrow_mut_data()?;

    // Load without enforcing the current struct layout over the old bytes
    // (safe since we validated discriminator and zero-filled the extension)
    let mut state = State::try_deserialize_unchecked(&mut &data[..])?;

    state.bump = bump;
    state.try_serialize(&mut &mut data[..])?;

    msg!("State migrated");
    Ok(())
}

fn migrate_permissionless(ctx: &Context<MigrateV3>) -> Result<()> {
    // Validations
    let bump = {
        let data = ctx.accounts.permissionless_authority.try_borrow_data()?;

        require!(
            data.len() == OLD_PERMISSIONLESS_SPACE || data.len() == NEW_PERMISSIONLESS_SPACE,
            MigrationError::InvalidPermissionlessData
        );
        let old_discriminator = &hash(b"account:PermissionlessAccount").to_bytes()[0..8];
        require!(
            data.starts_with(&PermissionlessAuthority::DISCRIMINATOR)
                || data.starts_with(old_discriminator),
            MigrationError::InvalidPermissionlessData
        );

        let (pda, bump) =
            Pubkey::find_program_address(&[seeds::PERMISSIONLESS_AUTHORITY], ctx.program_id);

        require_keys_eq!(
            pda,
            ctx.accounts.permissionless_authority.key(),
            MigrationError::InvalidPermissionlessData
        );

        bump
    }; // data borrow is dropped here

    top_up_lamports(
        ctx.accounts.permissionless_authority.to_account_info(),
        ctx.accounts.boss.to_account_info(),
        NEW_PERMISSIONLESS_SPACE,
        ctx.accounts.system_program.to_account_info(),
    )?;

    // Realloc
    ctx.accounts
        .permissionless_authority
        .realloc(NEW_PERMISSIONLESS_SPACE, true)?;

    // Now get a fresh mutable borrow after realloc
    let mut data = ctx
        .accounts
        .permissionless_authority
        .try_borrow_mut_data()?;

    // Load without enforcing the current struct layout over the old bytes
    // (safe since we validated discriminator and zero-filled the extension)
    let mut permissionless = PermissionlessAuthority::try_deserialize_unchecked(&mut &data[..])?;

    permissionless.bump = bump;
    permissionless.try_serialize(&mut &mut data[..])?;

    msg!("Permissionless migrated");
    Ok(())
}

fn top_up_lamports<'a>(
    account: AccountInfo<'a>,
    boss: AccountInfo<'a>,
    new_account_size: usize,
    system_program: AccountInfo<'a>,
) -> Result<()> {
    let rent = Rent::get()?;
    let want = rent.minimum_balance(new_account_size);
    let have = account.lamports();
    if have < want {
        let ix = system_instruction::transfer(&boss.key(), &account.key(), want - have);
        anchor_lang::solana_program::program::invoke(&ix, &[boss, account, system_program])?;
    }

    Ok(())
}
