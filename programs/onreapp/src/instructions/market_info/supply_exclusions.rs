//! Per-mint circulating-supply exclusion cache shared by the boss set, the
//! permissionless refresh and every modular reader.
//!
//! Balances cannot be read whenever circulating supply is needed (every excluded
//! account would have to be supplied), so the set and refresh cache their sum and
//! readers use only that cached amount. For ONyc the same amount is also written to
//! the legacy `CirculatingSupplyExcludedBalance` cache that legacy paths read.
use crate::{
    model::{seeds::SUPPLY_EXCLUSIONS, ManagedTokenSupplyExclusions},
    state::{CirculatingSupplyExcludedBalance, State},
    utils::{load_optional_pda_account, load_or_init_pda_account, store_pda_account},
    OnreError,
};
use anchor_lang::{prelude::*, system_program};
use anchor_spl::token_interface::{Mint, TokenAccount};

/// Emitted whenever the cached excluded amount is recomputed (set or refresh).
#[event]
pub struct ManagedTokenSupplyExclusionsUpdatedEvent {
    pub managed_mint: Pubkey,
    pub token_accounts: Vec<Pubkey>,
    pub excluded_amount: u64,
    /// Legacy ONyc cache written with the same amount; default for other mints.
    pub legacy_excluded_balance: Pubkey,
    pub updater: Pubkey,
    pub timestamp: i64,
    pub slot: u64,
}

pub fn supply_exclusions_address(program_id: &Pubkey, managed_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[SUPPLY_EXCLUSIONS, managed_mint.as_ref()], program_id).0
}

/// Supply excluded from a managed mint's circulating supply: the cached amount of
/// its `["supply_exclusions", mint]` account, capped at current supply so a stale
/// cache (e.g. excluded tokens burned since the last refresh) floors circulating
/// supply at zero instead of failing. A mint without that account excludes
/// nothing, except ONyc, which must have it so that modular writes of the legacy
/// market stats singleton never use unexcluded supply.
pub(crate) fn excluded_supply_amount(
    program_id: &Pubkey,
    state: &State,
    managed_mint: &InterfaceAccount<Mint>,
    supply_exclusions: &AccountInfo,
) -> Result<u64> {
    let managed_mint_key = managed_mint.key();
    let managed_mint_supply = managed_mint.supply;
    let managed_mint = &managed_mint_key;
    require_keys_eq!(
        supply_exclusions.key(),
        supply_exclusions_address(program_id, managed_mint),
        OnreError::InvalidCirculatingSupplyExcludedBalance
    );
    match load_optional_pda_account::<ManagedTokenSupplyExclusions>(
        supply_exclusions,
        program_id,
        OnreError::InvalidCirculatingSupplyExcludedBalanceOwner.into(),
        OnreError::InvalidCirculatingSupplyExcludedBalanceData.into(),
    )? {
        Some(exclusions) => Ok(exclusions.excluded_amount.min(managed_mint_supply)),
        None if *managed_mint == state.onyc_mint => err!(OnreError::SupplyExclusionsRequired),
        None => Ok(0),
    }
}

/// Sums token accounts of `mint` under its token program. A closed account (empty
/// and system-owned) counts as zero only when `allow_closed`, so the boss cannot
/// store a non-token address while a refresh is never blocked by a holder closing
/// an excluded account.
pub(crate) fn sum_balances(
    mint: &InterfaceAccount<Mint>,
    accounts: &[AccountInfo],
    allow_closed: bool,
) -> Result<u64> {
    let token_program = *mint.to_account_info().owner;
    let mut total = 0_u64;
    for account in accounts {
        if allow_closed && account.data_is_empty() && *account.owner == system_program::ID {
            continue;
        }
        require_keys_eq!(
            *account.owner,
            token_program,
            OnreError::InvalidExcludedTokenAccount
        );
        let token = TokenAccount::try_deserialize(&mut &account.try_borrow_data()?[..])
            .map_err(|_| error!(OnreError::InvalidExcludedTokenAccount))?;
        require_keys_eq!(
            token.mint,
            mint.key(),
            OnreError::InvalidExcludedTokenAccount
        );
        total = total
            .checked_add(token.amount)
            .ok_or(OnreError::MathOverflow)?;
    }
    Ok(total)
}

/// Stores `excluded_amount` with the current time, mirrors it into the legacy
/// ONyc cache when the mint is ONyc (creating it with `payer` if absent), and emits
/// the update event. `legacy_excluded_balance` is the seed-validated legacy cache
/// and its bump; callers' constraints require it for ONyc.
#[allow(clippy::too_many_arguments)]
pub(crate) fn record<'info>(
    program_id: &Pubkey,
    state: &State,
    exclusions: &mut ManagedTokenSupplyExclusions,
    excluded_amount: u64,
    legacy_excluded_balance: Option<(AccountInfo<'info>, u8)>,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    updater: Pubkey,
) -> Result<()> {
    let clock = Clock::get()?;
    exclusions.excluded_amount = excluded_amount;
    exclusions.last_updated_at = clock.unix_timestamp;
    exclusions.last_updated_slot = clock.slot;

    let legacy = if exclusions.managed_mint == state.onyc_mint {
        let (legacy, bump) =
            legacy_excluded_balance.ok_or(OnreError::InvalidCirculatingSupplyExcludedBalance)?;
        let mut cache = load_or_init_pda_account::<CirculatingSupplyExcludedBalance>(
            &legacy,
            payer,
            system_program,
            program_id,
            bump,
        )?;
        cache.amount = excluded_amount;
        cache.last_updated_at = clock.unix_timestamp;
        cache.last_updated_slot = clock.slot;
        cache.bump = bump;
        store_pda_account(&legacy, &cache)?;
        legacy.key()
    } else {
        Pubkey::default()
    };

    emit!(ManagedTokenSupplyExclusionsUpdatedEvent {
        managed_mint: exclusions.managed_mint,
        token_accounts: exclusions.active().to_vec(),
        excluded_amount,
        legacy_excluded_balance: legacy,
        updater,
        timestamp: clock.unix_timestamp,
        slot: clock.slot,
    });
    Ok(())
}
