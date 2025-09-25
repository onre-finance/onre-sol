use crate::state::State;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::Discriminator;

// Old size of State (accounting for the 8-byte discriminator).
// Old State only had: boss (32 bytes)
const OLD_STATE_SPACE: usize = 8 + 32;

// New size (with InitSpace the const excludes the 8-byte discriminator).
// New State has: boss (32 bytes) + is_killed (1 byte) + onyc_mint (32 bytes) + admins array (20 * 32 = 640 bytes)
const NEW_STATE_SPACE: usize = 8 + <State as Space>::INIT_SPACE;

#[error_code]
pub enum MigrationError {
    #[msg("Invalid State account data")]
    InvalidStateData,
    #[msg("Boss pubkey mismatch")]
    BossMismatch,
}

#[derive(Accounts)]
pub struct MigrateState<'info> {
    /// CHECK: handled manually to avoid deserializing the new struct over old data
    #[account(mut)]
    pub state: AccountInfo<'info>,

    /// The boss who is authorized to perform the migration
    #[account(mut)]
    pub boss: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn migrate_state(ctx: Context<MigrateState>) -> Result<()> {
    {
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
        let stored_boss = Pubkey::new_from_array(data[8..8 + 32].try_into().unwrap());
        require_keys_eq!(
            stored_boss,
            ctx.accounts.boss.key(),
            MigrationError::BossMismatch
        );
    }

    {
        let rent = Rent::get()?;
        let want = rent.minimum_balance(NEW_STATE_SPACE);
        let have = ctx.accounts.state.lamports();
        if have < want {
            let ix = system_instruction::transfer(
                &ctx.accounts.boss.key(),
                &ctx.accounts.state.key(),
                want - have,
            );
            anchor_lang::solana_program::program::invoke(
                &ix,
                &[
                    ctx.accounts.boss.to_account_info(),
                    ctx.accounts.state.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
        }
    }

    // 3) Realloc
    ctx.accounts.state.realloc(NEW_STATE_SPACE, true)?;

    msg!("State migrated: boss preserved, is_killed = false, onyc_mint = Pubkey::default(), admins = empty");
    Ok(())
}
