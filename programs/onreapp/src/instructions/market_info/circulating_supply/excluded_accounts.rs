use crate::constants::{seeds, MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS};
use crate::state::{CirculatingSupplyExcludedAccounts, State};
use crate::utils::PdaAccountInit;
use anchor_lang::prelude::*;

#[event]
pub struct CirculatingSupplyExcludedAccountsSetEvent {
    pub owners: [Pubkey; MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS],
    pub boss: Pubkey,
}

impl PdaAccountInit for CirculatingSupplyExcludedAccounts {
    fn pda_seed_prefixes() -> &'static [&'static [u8]] {
        &[seeds::CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS]
    }

    fn init_space() -> usize {
        8 + CirculatingSupplyExcludedAccounts::INIT_SPACE
    }

    fn init_value(bump: u8) -> Self {
        Self {
            owners: [Pubkey::default(); MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS],
            bump,
            reserved: [0; 31],
        }
    }

    fn invalid_owner_error() -> Error {
        error!(crate::OnreError::InvalidCirculatingSupplyExcludedAccountsOwner)
    }

    fn invalid_data_error() -> Error {
        error!(crate::OnreError::InvalidCirculatingSupplyExcludedAccountsData)
    }
}

#[derive(Accounts)]
pub struct SetCirculatingSupplyExcludedAccounts<'info> {
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ crate::OnreError::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    #[account(mut)]
    pub boss: Signer<'info>,

    #[account(
        init_if_needed,
        payer = boss,
        space = 8 + CirculatingSupplyExcludedAccounts::INIT_SPACE,
        seeds = [seeds::CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS],
        bump
    )]
    pub excluded_accounts: Account<'info, CirculatingSupplyExcludedAccounts>,

    pub system_program: Program<'info, System>,
}

pub fn set_circulating_supply_excluded_accounts(
    ctx: Context<SetCirculatingSupplyExcludedAccounts>,
    owners: [Pubkey; MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS],
) -> Result<()> {
    validate_unique_non_default_owners(&owners)?;

    let excluded_accounts = &mut ctx.accounts.excluded_accounts;
    excluded_accounts.owners = owners;
    excluded_accounts.bump = ctx.bumps.excluded_accounts;

    emit!(CirculatingSupplyExcludedAccountsSetEvent {
        owners,
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}

fn validate_unique_non_default_owners(
    owners: &[Pubkey; MAX_CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS],
) -> Result<()> {
    for (index, owner) in owners.iter().enumerate() {
        if *owner == Pubkey::default() {
            continue;
        }
        require!(
            !owners.iter().take(index).any(|previous| previous == owner),
            crate::OnreError::DuplicateExcludedAccountOwner
        );
    }

    Ok(())
}
