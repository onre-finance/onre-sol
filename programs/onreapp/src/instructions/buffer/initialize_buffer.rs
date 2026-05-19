use crate::constants::seeds;
use crate::instructions::buffer::{BufferInitializedEvent, BufferState};
use crate::instructions::Offer;
use crate::state::{ConfigurableVault, ConfigurableVaultKind, State};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct InitializeBuffer<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
        has_one = onyc_mint,
    )]
    pub state: Box<Account<'info, State>>,

    #[account(
        init,
        payer = boss,
        space = 8 + BufferState::INIT_SPACE,
        seeds = [seeds::BUFFER_STATE],
        bump
    )]
    pub buffer_state: Box<Account<'info, BufferState>>,

    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        init_if_needed,
        payer = boss,
        space = 8,
        seeds = [seeds::RESERVE_VAULT_AUTHORITY],
        bump
    )]
    pub reserve_vault_authority: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = boss,
        space = 8 + ConfigurableVault::INIT_SPACE,
        seeds = [seeds::CONFIGURABLE_VAULT, seeds::MANAGEMENT_FEE_VAULT],
        bump
    )]
    pub management_fee_vault: Account<'info, ConfigurableVault>,

    #[account(
        init_if_needed,
        payer = boss,
        space = 8 + ConfigurableVault::INIT_SPACE,
        seeds = [seeds::CONFIGURABLE_VAULT, seeds::PERFORMANCE_FEE_VAULT],
        bump
    )]
    pub performance_fee_vault: Account<'info, ConfigurableVault>,

    #[account(mut)]
    pub boss: Signer<'info>,

    pub onyc_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(address = state.main_offer @ crate::OnreError::InvalidMainOffer)]
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = onyc_mint,
        associated_token::authority = reserve_vault_authority,
        associated_token::token_program = token_program
    )]
    pub reserve_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = onyc_mint,
        associated_token::authority = management_fee_vault,
        associated_token::token_program = token_program
    )]
    pub management_fee_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = onyc_mint,
        associated_token::authority = performance_fee_vault,
        associated_token::token_program = token_program
    )]
    pub performance_fee_vault_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_buffer(ctx: Context<InitializeBuffer>) -> Result<()> {
    let buffer_state = &mut ctx.accounts.buffer_state;
    let now = Clock::get()?.unix_timestamp;
    let main_offer = ctx.accounts.offer.key();
    let offer = ctx.accounts.offer.load()?;
    require_keys_eq!(
        ctx.accounts.onyc_mint.key(),
        offer.token_out_mint,
        crate::OnreError::InvalidTokenOutMint
    );

    buffer_state.onyc_mint = ctx.accounts.onyc_mint.key();
    buffer_state.last_accrual_timestamp = now;
    buffer_state.bump = ctx.bumps.buffer_state;

    init_fee_vault(
        &mut ctx.accounts.management_fee_vault,
        ConfigurableVaultKind::ManagementFee,
        ctx.bumps.management_fee_vault,
    )?;
    init_fee_vault(
        &mut ctx.accounts.performance_fee_vault,
        ConfigurableVaultKind::PerformanceFee,
        ctx.bumps.performance_fee_vault,
    )?;

    emit!(BufferInitializedEvent {
        buffer_state: buffer_state.key(),
        onyc_mint: buffer_state.onyc_mint,
        main_offer,
        timestamp: now,
    });

    Ok(())
}

fn init_fee_vault(
    vault: &mut Account<ConfigurableVault>,
    kind: ConfigurableVaultKind,
    bump: u8,
) -> Result<()> {
    if vault.kind == 0 && vault.withdrawal_destination == Pubkey::default() && vault.bump == 0 {
        vault.kind = kind.as_u8();
        vault.bump = bump;
    }

    require!(
        vault.kind == kind.as_u8(),
        crate::OnreError::InvalidConfigurableVaultKind
    );

    Ok(())
}
