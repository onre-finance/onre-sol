use super::QuoterKind;
use anchor_lang::prelude::*;

/// Who may trade: approver-signed users only, or anyone.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferAccess {
    /// Requires an approver-signed approval for the trading user: on `take_offer`,
    /// or on `create_fulfillment_request` for Worker execution.
    Permissioned,
    /// No approval. Instant execution settles through the permissionless intermediary.
    Permissionless,
}

/// How a trade settles: atomically in `take_offer`, or as an escrowed request that
/// the worker fills, possibly partially. Each fill uses the offer's quoter, fee and
/// custody exactly as `take_offer` would, at the price current at that fill.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferExecution {
    Instant,
    Worker,
}

/// The quoters an access mode supports, checked on configuration and again on
/// execution: every offer may quote NAV; only permissionless offers may quote
/// Prop AMM. Permissioned and permissionless offers can share one NAV quoter and
/// differ by fee config. Execution mode and direction do not restrict the quoter:
/// a Worker fill prices and settles exactly as `take_offer` would.
pub fn quoter_supported(access: OfferAccess, kind: QuoterKind) -> bool {
    match kind {
        QuoterKind::Nav => true,
        QuoterKind::PropAmm => access == OfferAccess::Permissionless,
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferDirection {
    AssetToManaged,
    ManagedToAsset,
}

/// One immutable directed mint pair, access and execution mode, with replaceable
/// component references.
///
/// The boss selects the external asset. Pricer replacements must match the managed
/// mint and the currently selected Pricer's denomination.
#[account]
#[derive(InitSpace)]
pub struct OfferConfig {
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    /// Boss-chosen identity within the directed pair: the PDA is
    /// `["offer_config", token_in_mint, token_out_mint, instance_id_le]`, so any number
    /// of offers, with any access and execution modes, can exist per pair.
    pub instance_id: u64,
    pub access: OfferAccess,
    pub execution: OfferExecution,
    pub direction: OfferDirection,
    /// Explicit price source, validated against this offer's mint pair on assignment.
    pub pricer: Pubkey,
    pub quoter: Pubkey,
    /// Explicit pair pressure-accounting link, independent of quote selection.
    /// Default means untracked NAV; PropAmm requires its own canonical state.
    pub pressure_state: Pubkey,
    pub fee_config: Pubkey,
    pub proceeds_vault: Pubkey,
    /// Output-mint custody: pay from inventory when set, otherwise mint managed output.
    /// Mandatory for ManagedToAsset. Supports modular Liquidity, legacy configurable,
    /// or redemption custody; default means absent.
    pub source_vault: Pubkey,
    /// Input-mint custody: buy assets refill its target; managed sell input is
    /// transferred here instead of burned. Default means absent. PropAmm buys
    /// require it as their funded asset-side curve liquidity.
    pub destination_vault: Pubkey,
    /// Haircut custody for Prop AMM sells without a destination: the haircut share of
    /// the net managed input is transferred here and only the paid share is burned.
    /// Required for those offers and default (absent) for every other offer.
    pub haircut_vault: Pubkey,
    pub token_in_decimals: u8,
    pub token_out_decimals: u8,
    pub enabled: bool,
    pub bump: u8,
}
