use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferFlow {
    Permissioned,
    Permissionless,
    Worker,
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferDirection {
    AssetToManaged,
    ManagedToAsset,
}

/// One immutable directed mint pair and flow, with replaceable component references.
///
/// The boss selects the external asset. Pricer replacements must match the managed
/// mint and the currently selected Pricer's denomination.
#[account]
#[derive(InitSpace)]
pub struct OfferConfig {
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub flow: OfferFlow,
    pub direction: OfferDirection,
    /// Explicit price source, validated against this offer's mint pair on assignment.
    pub pricer: Pubkey,
    pub quoter: Pubkey,
    /// Explicit pair pressure-accounting link, independent of quote selection.
    /// Default means untracked NAV; PropAmm requires its own canonical state.
    pub pressure_state: Pubkey,
    pub fee_config: Pubkey,
    pub proceeds_vault: Pubkey,
    /// Modular Liquidity VaultConfig, legacy ConfigurableVault, or redemption authority.
    /// Default pubkey means absent; mandatory for ManagedToAsset.
    pub liquidity_vault: Pubkey,
    pub token_in_decimals: u8,
    pub token_out_decimals: u8,
    pub enabled: bool,
    pub bump: u8,
}
