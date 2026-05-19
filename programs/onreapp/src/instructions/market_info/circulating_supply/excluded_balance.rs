use crate::constants::seeds;
use crate::state::{CirculatingSupplyExcludedAccounts, CirculatingSupplyExcludedBalance, State};
use crate::utils::{load_optional_pda_account, PdaAccountInit};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event]
pub struct CirculatingSupplyExcludedBalanceUpdatedEvent {
    pub amount: u64,
    pub updater: Pubkey,
    pub timestamp: i64,
    pub slot: u64,
}

impl PdaAccountInit for CirculatingSupplyExcludedBalance {
    fn pda_seed_prefixes() -> &'static [&'static [u8]] {
        &[seeds::CIRCULATING_SUPPLY_EXCLUDED_BALANCE]
    }

    fn init_space() -> usize {
        8 + CirculatingSupplyExcludedBalance::INIT_SPACE
    }

    fn init_value(bump: u8) -> Self {
        Self {
            amount: 0,
            last_updated_at: 0,
            last_updated_slot: 0,
            bump,
            reserved: [0; 31],
        }
    }

    fn invalid_owner_error() -> Error {
        error!(crate::OnreError::InvalidCirculatingSupplyExcludedBalanceOwner)
    }

    fn invalid_data_error() -> Error {
        error!(crate::OnreError::InvalidCirculatingSupplyExcludedBalanceData)
    }
}

#[derive(Accounts)]
pub struct UpdateCirculatingSupplyExcludedBalance<'info> {
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = onyc_mint)]
    pub state: Box<Account<'info, State>>,

    pub onyc_mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [seeds::CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS],
        bump = excluded_accounts.bump
    )]
    pub excluded_accounts: Account<'info, CirculatingSupplyExcludedAccounts>,

    #[account(
        init_if_needed,
        payer = signer,
        space = 8 + CirculatingSupplyExcludedBalance::INIT_SPACE,
        seeds = [seeds::CIRCULATING_SUPPLY_EXCLUDED_BALANCE],
        bump
    )]
    pub excluded_balance: Account<'info, CirculatingSupplyExcludedBalance>,

    pub token_program: Interface<'info, TokenInterface>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn update_circulating_supply_excluded_balance(
    ctx: Context<UpdateCirculatingSupplyExcludedBalance>,
) -> Result<()> {
    let active_owners: Vec<Pubkey> = ctx
        .accounts
        .excluded_accounts
        .owners
        .iter()
        .copied()
        .filter(|owner| *owner != Pubkey::default())
        .collect();

    require!(
        ctx.remaining_accounts.len() <= active_owners.len(),
        crate::OnreError::TooManyExcludedTokenAccounts
    );
    require!(
        ctx.remaining_accounts.len() == active_owners.len(),
        crate::OnreError::MissingExcludedTokenAccount
    );

    let mut amount = 0_u64;
    for (owner, token_account_info) in active_owners.iter().zip(ctx.remaining_accounts.iter()) {
        let expected_ata = get_associated_token_address_with_program_id(
            owner,
            &ctx.accounts.onyc_mint.key(),
            &ctx.accounts.token_program.key(),
        );
        require_keys_eq!(
            token_account_info.key(),
            expected_ata,
            crate::OnreError::InvalidExcludedTokenAccount
        );

        let token_account = InterfaceAccount::<TokenAccount>::try_from(token_account_info)?;
        require_keys_eq!(
            token_account.mint,
            ctx.accounts.onyc_mint.key(),
            crate::OnreError::InvalidExcludedTokenAccount
        );
        require_keys_eq!(
            token_account.owner,
            *owner,
            crate::OnreError::InvalidExcludedTokenAccount
        );

        amount = amount
            .checked_add(token_account.amount)
            .ok_or(crate::OnreError::MathOverflow)?;
    }

    let clock = Clock::get()?;
    let excluded_balance = &mut ctx.accounts.excluded_balance;
    excluded_balance.amount = amount;
    excluded_balance.last_updated_at = clock.unix_timestamp;
    excluded_balance.last_updated_slot = clock.slot;
    excluded_balance.bump = ctx.bumps.excluded_balance;

    emit!(CirculatingSupplyExcludedBalanceUpdatedEvent {
        amount,
        updater: ctx.accounts.signer.key(),
        timestamp: clock.unix_timestamp,
        slot: clock.slot,
    });

    Ok(())
}

pub fn load_circulating_supply_excluded_balance_amount(
    program_id: &Pubkey,
    excluded_balance_account: &AccountInfo,
) -> Result<u64> {
    let (expected_excluded_balance, _) =
        Pubkey::find_program_address(&[seeds::CIRCULATING_SUPPLY_EXCLUDED_BALANCE], program_id);
    require_keys_eq!(
        excluded_balance_account.key(),
        expected_excluded_balance,
        crate::OnreError::InvalidCirculatingSupplyExcludedBalance
    );

    let Some(excluded_balance) = load_optional_pda_account::<CirculatingSupplyExcludedBalance>(
        excluded_balance_account,
        program_id,
        crate::OnreError::InvalidCirculatingSupplyExcludedBalanceOwner.into(),
        crate::OnreError::InvalidCirculatingSupplyExcludedBalanceData.into(),
    )?
    else {
        return Ok(0);
    };

    Ok(excluded_balance.amount)
}
