//! Boss-only burn of per-mint BUFFER reserve tokens to raise NAV, the per-mint
//! port of the legacy ONyc `burn_for_nav_increase` and EVM `burnForNavIncrease`.
use super::accounts::{
    __client_accounts_managed_buffer_accounts, __cpi_client_accounts_managed_buffer_accounts,
    ManagedBufferAccountsBumps,
};
use super::{accrue, refresh_market_stats, ManagedBufferAccounts};
use crate::{
    instructions::{
        buffer::{burn_for_nav_increase::calculate_burn_amount, modular_accrual},
        market_info::{
            calculate_circulating_supply, calculate_tvl, excluded_supply_amount,
            is_excluded_token_account,
        },
    },
    model::seeds::BUFFER_RESERVE,
    utils::{pricing::get_active_vector_and_current_price, read_optional_token_account_amount},
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, BurnChecked};

#[event]
pub struct ManagedTokenBurnedForNavEvent {
    pub boss: Pubkey,
    pub managed_mint: Pubkey,
    pub burn_amount: u64,
    /// USD value removed from the asset base, in managed-mint base units (scale of TVL).
    pub asset_adjustment_amount: u64,
    /// Circulating supply × NAV before the burn.
    pub total_assets: u64,
    /// NAV preserved by the burn, scale 9.
    pub nav: u64,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct BurnManagedTokenForNavIncrease<'info> {
    pub buffer_accounts: ManagedBufferAccounts<'info>,

    #[account(address = buffer_accounts.state.boss @ OnreError::InvalidBoss)]
    pub boss: Signer<'info>,

    /// CHECK: The BUFFER's `["buffer_reserve", mint]` vault, which signs the burn of
    /// its reserve ATA; must also equal the reserve vault stored in the BUFFER state.
    #[account(seeds = [BUFFER_RESERVE, buffer_accounts.managed_mint.key().as_ref()], bump)]
    pub reserve_vault: UncheckedAccount<'info>,

    /// CHECK: Must equal `ManagedTokenConfig::market_stats` when one is selected;
    /// ignored otherwise.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: Legacy ONyc `["market_stats"]` singleton. Required when the managed
    /// mint is ONyc and refreshed once it exists; ignored for other mints.
    #[account(mut)]
    pub legacy_market_stats: Option<UncheckedAccount<'info>>,

    /// CHECK: The mint's `["supply_exclusions", mint]` cache, validated where read;
    /// may be absent except for ONyc.
    pub supply_exclusions: UncheckedAccount<'info>,
}

/// Accrues the BUFFER, then burns reserve tokens so that the asset base reduced by
/// `asset_adjustment_amount` (USD in managed-mint base units) keeps the current
/// canonical NAV:
///
/// ```text
/// total_assets = floor(circulating × NAV / 1e9)
/// required     = ceil((total_assets − adjustment) × 1e9 / NAV)
/// burn         = circulating − required
/// ```
///
/// Rejects a zero adjustment, a reserve token account in the supply exclusions, an
/// adjustment above total assets, a zero burn and an insufficient reserve. The BUFFER baseline and market stats follow the burn.
pub fn burn_managed_token_for_nav_increase(
    ctx: Context<BurnManagedTokenForNavIncrease>,
    asset_adjustment_amount: u64,
) -> Result<()> {
    require!(
        asset_adjustment_amount > 0,
        OnreError::InvalidAssetAdjustmentAmount
    );
    let now = Clock::get()?.unix_timestamp;
    let program_id = ctx.program_id;
    let a = ctx.accounts;
    accrue(
        program_id,
        &mut a.buffer_accounts,
        ctx.bumps.buffer_accounts.mint_authority,
        now,
    )?;

    let b = &a.buffer_accounts;
    let mint = b.managed_mint.key();
    let reserve_bump = ctx.bumps.reserve_vault;
    require_keys_eq!(
        modular_accrual::reserve_vault(program_id, &b.context())?,
        a.reserve_vault.key(),
        OnreError::InvalidConfigurableVault
    );

    let valuation = modular_accrual::canonical_market_valuation(program_id, &b.context())?;
    let nav = get_active_vector_and_current_price(valuation.vectors, now as u64)?.1;
    let excluded =
        excluded_supply_amount(program_id, &b.state, &b.managed_mint, &a.supply_exclusions)?;
    // Burning an excluded reserve would leave circulating supply unchanged (EVM parity).
    require!(
        !is_excluded_token_account(program_id, &a.supply_exclusions, &b.reserve_token.key())?,
        OnreError::BufferReserveExcludedFromSupply
    );
    let circulating = calculate_circulating_supply(b.managed_mint.supply, excluded);
    let total_assets = calculate_tvl(circulating, nav)?;
    let burn_amount = calculate_burn_amount(circulating, asset_adjustment_amount, nav, nav)?;
    // The reserve ATA was validated against the stored reserve vault during accrual.
    require!(
        burn_amount <= read_optional_token_account_amount(&b.reserve_token, &b.token_program)?,
        OnreError::InsufficientCacheBalance
    );
    token_interface::burn_checked(
        CpiContext::new_with_signer(
            b.token_program.key(),
            BurnChecked {
                mint: b.managed_mint.to_account_info(),
                from: b.reserve_token.to_account_info(),
                authority: a.reserve_vault.to_account_info(),
            },
            &[&[BUFFER_RESERVE, mint.as_ref(), &[reserve_bump]]],
        ),
        burn_amount,
        b.managed_mint.decimals,
    )?;

    a.buffer_accounts.managed_mint.reload()?;
    modular_accrual::store_post_supply(program_id, &a.buffer_accounts.context(), now)?;
    refresh_market_stats(
        program_id,
        &a.buffer_accounts,
        a.market_stats.to_account_info(),
        a.legacy_market_stats
            .as_ref()
            .map(|account| account.to_account_info()),
        a.supply_exclusions.to_account_info(),
    )?;
    emit!(ManagedTokenBurnedForNavEvent {
        boss: a.boss.key(),
        managed_mint: mint,
        burn_amount,
        asset_adjustment_amount,
        total_assets,
        nav,
        timestamp: now,
    });
    Ok(())
}
