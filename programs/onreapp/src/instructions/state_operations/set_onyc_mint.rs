use crate::account;
use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Event emitted when the ONyc token mint is successfully updated
///
/// Provides transparency for tracking ONyc mint configuration changes.
#[event]
pub struct ONycMintUpdatedEvent {
    /// The previous ONyc mint public key before the update
    pub old_onyc_mint: Pubkey,
    /// The new ONyc mint public key after the update
    pub new_onyc_mint: Pubkey,
}

/// Account structure for configuring the ONyc token mint
///
/// This struct defines the accounts required to set or update the ONyc token
/// mint address in the program state. Only the boss can configure this setting.
#[derive(Accounts)]
pub struct SetOnycMint<'info> {
    /// Program state account containing the ONyc mint configuration
    ///
    /// Must be mutable to allow ONyc mint updates and have the boss account
    /// as the authorized signer for mint configuration management.
    #[account(
        mut,
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss
    )]
    pub state: Account<'info, State>,

    /// The boss account authorized to configure the ONyc mint
    pub boss: Signer<'info>,

    /// The ONyc token mint account to be set in program state
    pub onyc_mint: InterfaceAccount<'info, Mint>,
}

/// Configures the ONyc token mint address in program state
///
/// This instruction allows the boss to set or update the ONyc token mint that
/// the program recognizes for operations. The ONyc mint is used for calculating
/// market metrics and token-related operations within the protocol.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If the ONyc mint is successfully configured
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Updates the program state's onyc_mint field
/// - Configures which token mint is recognized as ONyc
/// - Affects future market calculations and operations
///
/// # Events
/// * `ONycMintUpdated` - Emitted with old and new ONyc mint addresses
pub fn set_onyc_mint(ctx: Context<SetOnycMint>) -> Result<()> {
    let state = &mut ctx.accounts.state;

    let old_onyc_mint = state.onyc_mint;
    state.onyc_mint = ctx.accounts.onyc_mint.key();

    msg!("ONyc mint updated: {}", state.onyc_mint);
    emit!(ONycMintUpdatedEvent {
        old_onyc_mint,
        new_onyc_mint: state.onyc_mint,
    });

    Ok(())
}
