//! Boss-only per-mint issuance limits for program-controlled modular minting.
use crate::{
    constants::seeds::STATE,
    model::{seeds::MANAGED_TOKEN, ManagedTokenConfig},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

#[event]
pub struct ManagedTokenMintLimitsSetEvent {
    pub boss: Pubkey,
    pub managed_mint: Pubkey,
    pub previous_max_supply: u64,
    pub previous_max_mint_amount: u64,
    pub max_supply: u64,
    pub max_mint_amount: u64,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct SetManagedTokenMintLimits<'info> {
    #[account(
        mut,
        seeds = [MANAGED_TOKEN, managed_mint.key().as_ref()],
        bump = managed_token.bump,
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    /// Supplies the current supply that a nonzero `max_supply` must not undercut.
    pub managed_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        has_one = boss @ OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    pub boss: Signer<'info>,
}

/// Sets the managed token's issuance limits in mint base units; zero disables a
/// limit. A nonzero `max_supply` must be at least the current supply, as on EVM.
/// The limits bind program mints (offer output and BUFFER accrual); for ONyc the
/// stricter nonzero of these and the global `State` limits applies. Mints by other
/// members of a mint multisig (e.g. a CCIP pool) are not limited by the program.
pub fn set_managed_token_mint_limits(
    ctx: Context<SetManagedTokenMintLimits>,
    max_supply: u64,
    max_mint_amount: u64,
) -> Result<()> {
    let a = ctx.accounts;
    let previous_max_supply = a.managed_token.max_supply;
    let previous_max_mint_amount = a.managed_token.max_mint_amount;
    require!(
        (max_supply, max_mint_amount) != (previous_max_supply, previous_max_mint_amount),
        OnreError::NoChange
    );
    require!(
        max_supply == 0 || max_supply >= a.managed_mint.supply,
        OnreError::MaxSupplyBelowCurrentSupply
    );
    a.managed_token.max_supply = max_supply;
    a.managed_token.max_mint_amount = max_mint_amount;
    emit!(ManagedTokenMintLimitsSetEvent {
        boss: a.boss.key(),
        managed_mint: a.managed_token.managed_mint,
        previous_max_supply,
        previous_max_mint_amount,
        max_supply,
        max_mint_amount,
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
