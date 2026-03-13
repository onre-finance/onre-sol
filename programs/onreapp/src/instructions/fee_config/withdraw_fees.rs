use crate::constants::seeds;
use crate::state::State;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use super::fee_config_state::{FeeConfig, FeeType};

/// Event emitted when fees are withdrawn from a FeeConfig PDA.
#[event]
pub struct FeesWithdrawnEvent {
    /// The FeeType discriminator
    pub fee_type: u8,
    /// Amount of tokens withdrawn
    pub amount: u64,
    /// Token mint of the withdrawn fees
    pub mint: Pubkey,
    /// The boss who received the fees
    pub boss: Pubkey,
}

/// Account structure for withdrawing accumulated fees from a FeeConfig PDA's ATA.
///
/// Transfers tokens from the FeeConfig PDA's associated token account to the
/// boss's ATA. The FeeConfig PDA signs the transfer via its derived seeds.
/// This is only useful when `destination` is `None` (fees accumulate in the PDA's ATA).
#[derive(Accounts)]
#[instruction(fee_type: FeeType)]
pub struct WithdrawFees<'info> {
    /// Program state — used to verify boss authority.
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ WithdrawFeesError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    /// The FeeConfig PDA that owns the source token account.
    #[account(
        seeds = [seeds::FEE_CONFIG, &[fee_type as u8]],
        bump = fee_config.bump
    )]
    pub fee_config: Account<'info, FeeConfig>,

    /// The FeeConfig PDA's associated token account holding accumulated fees.
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = fee_config,
        associated_token::token_program = token_program
    )]
    pub fee_config_token_account: InterfaceAccount<'info, TokenAccount>,

    /// The boss's associated token account to receive the withdrawn fees.
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = boss,
        associated_token::token_program = token_program
    )]
    pub boss_token_account: InterfaceAccount<'info, TokenAccount>,

    /// The token mint for the fees being withdrawn.
    pub mint: InterfaceAccount<'info, Mint>,

    /// SPL Token program for the transfer.
    pub token_program: Interface<'info, TokenInterface>,

    /// The boss authority.
    pub boss: Signer<'info>,
}

/// Withdraws accumulated fees from a FeeConfig PDA's ATA to the boss.
///
/// The FeeConfig PDA signs the transfer using its derived seeds
/// `[FEE_CONFIG, &[fee_type], &[bump]]`.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `fee_type` - The operation type to withdraw fees from
/// * `amount` - Amount of tokens to withdraw
///
/// # Access Control
/// - Boss only
pub fn withdraw_fees(
    ctx: Context<WithdrawFees>,
    fee_type: FeeType,
    amount: u64,
) -> Result<()> {
    let fee_type_discriminator = fee_type as u8;
    let bump = ctx.accounts.fee_config.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        seeds::FEE_CONFIG,
        std::slice::from_ref(&fee_type_discriminator),
        std::slice::from_ref(&bump),
    ]];

    transfer_tokens(
        &ctx.accounts.mint,
        &ctx.accounts.token_program,
        &ctx.accounts.fee_config_token_account,
        &ctx.accounts.boss_token_account,
        &ctx.accounts.fee_config.to_account_info(),
        Some(signer_seeds),
        amount,
    )?;

    msg!(
        "Fees withdrawn: type={}, amount={}, to={}",
        ctx.accounts.fee_config.fee_type,
        amount,
        ctx.accounts.boss.key()
    );

    emit!(FeesWithdrawnEvent {
        fee_type: fee_type_discriminator,
        amount,
        mint: ctx.accounts.mint.key(),
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

#[error_code]
pub enum WithdrawFeesError {
    #[msg("Invalid boss account")]
    InvalidBoss,
}