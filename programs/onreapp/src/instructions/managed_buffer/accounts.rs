use crate::{
    constants::seeds,
    instructions::buffer::modular_accrual::BufferContext,
    model::{seeds::MANAGED_TOKEN, ManagedTokenConfig, Pricer},
    state::State,
    utils::program_controls_mint,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

/// Shared modular BUFFER accounts. Every managed mint, ONyc included, uses its
/// per-mint state and vault ATAs, independent of the legacy singleton BUFFER.
#[derive(Accounts)]
pub struct ManagedBufferAccounts<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = !state.is_killed @ OnreError::KillSwitchActivated,
    )]
    pub state: Box<Account<'info, State>>,

    #[account(
        seeds = [MANAGED_TOKEN, managed_mint.key().as_ref()],
        bump = managed_token.bump,
        constraint = managed_token.enabled @ OnreError::ConfigurationDisabled,
        constraint = managed_token.managed_mint == managed_mint.key() @ OnreError::InvalidMint,
        constraint = managed_token.decimals == managed_mint.decimals @ OnreError::InvalidMint,
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    #[account(
        mut,
        mint::token_program = token_program,
        constraint = program_controls_mint(&managed_mint, &mint_authority)
            @ OnreError::ProgramNotMintAuthority,
    )]
    pub managed_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: Canonical mint signing identity; mint authority checked above.
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        bump,
    )]
    pub mint_authority: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    /// CHECK: Handler validates canonical `["buffer", mint]` identity, owner/discriminator
    /// and embedded mint. Standalone operations require initialized state.
    #[account(mut)]
    pub buffer_state: UncheckedAccount<'info>,

    /// Canonical USD instance-zero Pricer driving accrual and reporting valuation.
    pub buffer_pricer: Option<Box<Account<'info, Pricer>>>,

    /// CHECK: Handler validates the active reserve ATA before accrual.
    #[account(mut)]
    pub reserve_token: UncheckedAccount<'info>,

    /// CHECK: Handler validates the active management-fee ATA before accrual.
    #[account(mut)]
    pub management_token: UncheckedAccount<'info>,

    /// CHECK: Handler validates the active performance-fee ATA before accrual.
    #[account(mut)]
    pub performance_token: UncheckedAccount<'info>,
}

impl<'info> ManagedBufferAccounts<'info> {
    pub(super) fn context(&self) -> BufferContext<'_, 'info> {
        BufferContext {
            managed_mint: &self.managed_mint,
            managed_program: &self.token_program,
            buffer_state: &self.buffer_state,
            buffer_pricer: self.buffer_pricer.as_deref(),
            reserve_token: &self.reserve_token,
            management_token: &self.management_token,
            performance_token: &self.performance_token,
            mint_authority: &self.mint_authority,
        }
    }
}
