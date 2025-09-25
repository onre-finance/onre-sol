use crate::account;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Event emitted when the ONyc mint is updated in the program state.
#[event]
pub struct ONycMintUpdated {
    /// The previous ONyc mint stored in state.
    pub old_onyc_mint: Pubkey,
    /// The new ONyc mint.
    pub new_onyc_mint: Pubkey,
}

#[derive(Accounts)]
pub struct SetOnycMint<'info> {
    /// The program state account, containing the current onyc_mint to be updated.
    #[account(mut, has_one = boss)]
    pub state: Account<'info, State>,

    /// The boss who is authorized to perform the operation
    pub boss: Signer<'info>,

    /// The ONyc token mint
    pub onyc_mint: InterfaceAccount<'info, Mint>,
}

/// Sets the ONyc token mint in the program state.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the state update.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn set_onyc_mint(ctx: Context<SetOnycMint>) -> Result<()> {
    let state = &mut ctx.accounts.state;

    let old_onyc_mint = state.onyc_mint;
    state.onyc_mint = ctx.accounts.onyc_mint.key();

    msg!("ONyc mint updated: {}", state.onyc_mint);
    emit!(ONycMintUpdated {
        old_onyc_mint,
        new_onyc_mint: state.onyc_mint,
    });

    Ok(())
}
