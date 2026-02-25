use crate::constants::seeds;
use crate::instructions::cache::{CacheInitializedEvent, CacheState};
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct InitializeCache<'info> {
    #[account(
        mut,
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
        has_one = onyc_mint,
    )]
    pub state: Account<'info, State>,

    #[account(
        init,
        payer = boss,
        space = 8 + CacheState::INIT_SPACE,
        seeds = [seeds::CACHE_STATE],
        bump
    )]
    pub cache_state: Account<'info, CacheState>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        init_if_needed,
        payer = boss,
        space = 8,
        seeds = [seeds::CACHE_VAULT_AUTHORITY],
        bump
    )]
    pub cache_vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub boss: Signer<'info>,

    #[account(mut)]
    pub onyc_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = onyc_mint,
        associated_token::authority = cache_vault_authority,
        associated_token::token_program = token_program
    )]
    pub cache_vault_onyc_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_cache(ctx: Context<InitializeCache>, cache_admin: Pubkey) -> Result<()> {
    let cache_state = &mut ctx.accounts.cache_state;
    let now = Clock::get()?.unix_timestamp;

    cache_state.onyc_mint = ctx.accounts.onyc_mint.key();
    cache_state.cache_admin = cache_admin;
    cache_state.last_accrual_timestamp = now;
    cache_state.bump = ctx.bumps.cache_state;

    emit!(CacheInitializedEvent {
        cache_state: cache_state.key(),
        onyc_mint: cache_state.onyc_mint,
        cache_admin,
        timestamp: now,
    });

    Ok(())
}
