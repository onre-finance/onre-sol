use crate::constants::seeds;
use crate::state::{PermissionlessAuthority, State};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::Discriminator;

/// Event emitted when accounts are successfully migrated to v3
///
/// Provides transparency for tracking migration operations.
#[event]
pub struct AccountsMigratedEvent {
    /// The boss who performed the migration
    pub boss: Pubkey,
}

/// Size of the old State account including 8-byte discriminator
/// Old State structure only contained: boss (32 bytes)
const OLD_STATE_SPACE: usize = 8 + 32;

/// Size of the new State account including 8-byte discriminator
/// New State contains: boss (32 bytes) + is_killed (1 byte) + onyc_mint (32 bytes) +
/// admins array (20 * 32 = 640 bytes) + approver (32 bytes) + bump (1 byte) + reserved (128 bytes)
const NEW_STATE_SPACE: usize = 8 + <State as Space>::INIT_SPACE;

/// Size of the old PermissionlessAuthority account including 8-byte discriminator
/// Old structure was empty except for the discriminator
const OLD_PERMISSIONLESS_SPACE: usize = 8 + 4 + 50;

/// Size of the new PermissionlessAuthority account including 8-byte discriminator
/// New structure contains: name (String with max 50 chars) + bump (1 byte)
const NEW_PERMISSIONLESS_SPACE: usize = 8 + <PermissionlessAuthority as Space>::INIT_SPACE;

/// Error codes for migration operations
#[error_code]
pub enum MigrationError {
    /// State account has invalid size or discriminator
    #[msg("Invalid State account data")]
    InvalidStateData,
    /// PermissionlessAuthority account has invalid size or discriminator
    #[msg("Invalid PermissionlessAuthority account data")]
    InvalidPermissionlessData,
    /// The stored boss public key does not match the transaction signer
    #[msg("Boss pubkey mismatch")]
    BossMismatch,
}

/// Account structure for migrating program accounts to v3 layouts
///
/// This struct defines the accounts required for the v3 migration process
/// which upgrades State and PermissionlessAuthority accounts to their new
/// expanded layouts with additional fields and functionality.
#[derive(Accounts)]
pub struct MigrateV3<'info> {
    /// State account to be migrated from old to new layout
    ///
    /// Handled manually to avoid deserialization conflicts between old and new structures.
    /// Will be expanded from 40 bytes to include new fields for enhanced functionality.
    /// CHECK: Manual validation of discriminator and size performed in migration logic
    #[account(mut)]
    pub state: AccountInfo<'info>,

    /// PermissionlessAuthority account to be migrated from old to new layout
    ///
    /// Handled manually to avoid deserialization conflicts between old and new structures.
    /// Will be expanded from 8 bytes to include bump and name fields.
    /// CHECK: Manual validation of discriminator and size performed in migration logic
    #[account(mut)]
    pub permissionless_authority: AccountInfo<'info>,

    /// The boss account authorized to perform the migration and pay for additional rent
    #[account(mut)]
    pub boss: Signer<'info>,

    /// System program for account reallocation and rent transfers
    pub system_program: Program<'info, System>,
}

/// Migrates program accounts from v2 to v3 layouts with enhanced functionality
///
/// This instruction performs a comprehensive migration of both State and PermissionlessAuthority
/// accounts to their new v3 layouts, expanding their storage capacity and adding new fields
/// while preserving existing data integrity.
///
/// The migration expands State from 40 bytes to include admin management, kill switch,
/// and approval authority features. PermissionlessAuthority is expanded to include
/// bump and name fields for improved functionality.
///
/// # Arguments
/// * `ctx` - The migration context containing accounts to be migrated
///
/// # Returns
/// * `Ok(())` - If both account migrations complete successfully
/// * `Err(MigrationError::InvalidStateData)` - If State account validation fails
/// * `Err(MigrationError::InvalidPermissionlessData)` - If PermissionlessAuthority validation fails
/// * `Err(MigrationError::BossMismatch)` - If signer is not the stored boss
///
/// # Access Control
/// - Only the current boss stored in State account can perform migration
/// - Boss validation occurs during State account processing
///
/// # Effects
/// - Expands State account with new fields (is_killed, onyc_mint, admins, approver, bump)
/// - Expands PermissionlessAuthority account with bump and name fields
/// - Preserves existing boss public key in State
/// - Pays additional rent from boss account for increased storage
pub fn migrate_v3(ctx: Context<MigrateV3>) -> Result<()> {
    migrate_state(&ctx)?; // Signer == boss is validated inside migrate_state
    migrate_permissionless(&ctx)?;

    emit!(AccountsMigratedEvent {
        boss: ctx.accounts.boss.key(),
    });

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

/// Ensures an account has sufficient lamports for its new size requirement
///
/// Calculates the minimum rent required for the expanded account size and transfers
/// additional lamports from the boss if the current balance is insufficient.
/// This prevents rent exemption violations during account reallocation.
///
/// # Arguments
/// * `account` - The account being resized that may need additional rent
/// * `boss` - The boss account that will cover any additional rent costs
/// * `new_account_size` - The target size in bytes after reallocation
/// * `system_program` - The system program for executing lamport transfers
///
/// # Returns
/// * `Ok(())` - If account has sufficient rent or top-up transfer succeeds
/// * `Err(_)` - If rent calculation or transfer fails
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
