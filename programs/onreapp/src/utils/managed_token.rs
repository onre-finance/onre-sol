//! Token-wide validation shared by trading, BUFFER settlement and reporting.
use crate::{
    model::{seeds::PRICER, ManagedTokenConfig, Pricer, PricingDenomination},
    state::State,
    OnreError,
};
use anchor_lang::prelude::*;

pub(crate) fn canonical_usd_pricer_address(program_id: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            PRICER,
            mint.as_ref(),
            &[PricingDenomination::Usd as u8],
            &0_u64.to_le_bytes(),
        ],
        program_id,
    )
    .0
}

pub(crate) fn validate_canonical_usd_pricer(
    program_id: &Pubkey,
    mint: &Pubkey,
    pricer: &Account<Pricer>,
) -> Result<()> {
    require_keys_eq!(
        pricer.key(),
        canonical_usd_pricer_address(program_id, mint),
        OnreError::IncompatiblePricer
    );
    require_usd_pricer_for(mint, pricer)
}

/// Field checks for a Pricer whose canonical address the caller already verified.
pub(crate) fn require_usd_pricer_for(mint: &Pubkey, pricer: &Pricer) -> Result<()> {
    require!(
        pricer.managed_mint == *mint
            && pricer.denomination == PricingDenomination::Usd
            && pricer.enabled,
        OnreError::IncompatiblePricer
    );
    Ok(())
}

/// Zero disables a cap. ONyc retains the stricter nonzero global/per-mint cap.
pub(crate) fn effective_mint_limits(state: &State, managed: &ManagedTokenConfig) -> (u64, u64) {
    let onyc = managed.managed_mint == state.onyc_mint;
    let stricter = |a: u64, b: u64| match (a, b) {
        (0, x) | (x, 0) => x,
        (a, b) => a.min(b),
    };
    (
        stricter(managed.max_supply, if onyc { state.max_supply } else { 0 }),
        stricter(
            managed.max_mint_amount,
            if onyc { state.max_mint_amount } else { 0 },
        ),
    )
}
