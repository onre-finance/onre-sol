use crate::constants::seeds;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;

/// Event emitted when the state account is successfully closed
///
/// Provides transparency for tracking the closure of the program's main state account.
#[event]
pub struct StateClosedEvent {
    /// The PDA address of the state account that was closed
    pub state_pda: Pubkey,
    /// The boss account that initiated the closure and received the rent
    pub boss: Pubkey,
}

/// Account structure for closing the program state account
///
/// This struct defines the accounts required to permanently close the program's
/// main state account and transfer its rent balance back to the boss.
/// Only the boss can close the state account.
///
/// Note: The state account is NOT deserialized to allow closing accounts with
/// incompatible or outdated State structures.
#[derive(Accounts)]
pub struct CloseState<'info> {
    /// The state account to be closed and its rent reclaimed
    ///
    /// This account is validated as a PDA derived from the "state" seed.
    /// The account will be closed and its rent transferred to the boss.
    ///
    /// CHECK: Manual validation of PDA and boss without deserialization
    #[account(mut)]
    pub state: AccountInfo<'info>,

    /// The boss account authorized to close the state and receive rent
    ///
    /// Must match the boss stored in the state account.
    /// This signer will receive the rent from the closed state account.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// System program required for account closure and rent transfer
    pub system_program: Program<'info, System>,
}

/// Permanently closes the program's state account and reclaims its rent balance
///
/// This instruction removes the program's main state account and transfers its rent
/// balance back to the boss. The state account is permanently deleted and cannot
/// be recovered. All program configuration and governance settings are lost.
///
/// This operation effectively disables the program, as most instructions require
/// the state account to function. Use with extreme caution.
///
/// The state account is NOT deserialized, allowing this instruction to work even
/// when the on-chain State structure doesn't match the current program's State definition.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If the state is successfully closed and rent reclaimed
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - State account is permanently deleted
/// - Rent balance is transferred to the boss
/// - Program becomes effectively non-functional
///
/// # Events
/// * `StateClosedEvent` - Emitted with state PDA and boss details
pub fn close_state(ctx: Context<CloseState>) -> Result<()> {
    let state = &ctx.accounts.state;

    // 0) Sanity: we must own the account to mutate lamports/metadata directly
    require_keys_eq!(
        *state.owner,
        crate::ID,
        CloseStateErrorCode::InvalidStateOwner
    );

    // 1) Validate PDA address from seeds
    let (expected_state_pda, _bump) = Pubkey::find_program_address(&[seeds::STATE], &crate::ID);
    require_keys_eq!(
        state.key(),
        expected_state_pda,
        CloseStateErrorCode::InvalidStatePda
    );

    // 2) Read the stored boss pubkey from raw bytes (no deserialize),
    //    ensure we DROP the data borrow before mutating the account later.
    //    Layout: [8-byte discriminator][32-byte boss][...]
    let stored_boss = {
        let data = state.try_borrow_data()?;
        require!(data.len() >= 40, CloseStateErrorCode::InvalidStateData);

        // bytes 8..40 -> boss pubkey
        let arr: [u8; 32] = data[8..40]
            .try_into()
            .map_err(|_| error!(CloseStateErrorCode::InvalidStateData))?;
        Pubkey::new_from_array(arr)
    };

    let boss = &ctx.accounts.boss.to_account_info();

    // 3) Verify signer matches stored boss
    require_keys_eq!(
        boss.key(),
        stored_boss,
        CloseStateErrorCode::UnauthorizedSigner
    );

    // 4) Drain lamports safely (checked math), then zero the source.
    let state_lamports = state.lamports();

    // add first (checked), then zero state
    let boss_lamports_before = boss.lamports();
    let boss_lamport_after = boss_lamports_before
        .checked_add(state_lamports)
        .ok_or_else(|| error!(CloseStateErrorCode::LamportOverflow))?;
    **boss.try_borrow_mut_lamports()? = boss_lamport_after;
    **state.try_borrow_mut_lamports()? = 0;

    // 5) Deallocate & hand ownership back to System Program
    // (Make sure we no longer hold any data borrows at this point.)
    state.resize(0)?;
    state.assign(&system_program::ID);

    emit!(StateClosedEvent {
        state_pda: state.key(),
        boss: boss.key(),
    });

    Ok(())
}

/// Error codes for close state operations
#[error_code]
pub enum CloseStateErrorCode {
    /// State account is not owned by this program
    #[msg("State account must be owned by this program")]
    InvalidStateOwner,
    /// State account is not the expected PDA
    #[msg("Invalid state PDA")]
    InvalidStatePda,
    /// State account data is invalid or too short
    #[msg("Invalid state account data")]
    InvalidStateData,
    /// Signer is not the boss stored in state
    #[msg("Only the boss can close the state")]
    UnauthorizedSigner,
    /// Lamport arithmetic overflow
    #[msg("Lamport overflow")]
    LamportOverflow,
}
