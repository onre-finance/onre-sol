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
/// Pricer replacements must match the managed mint and the asset's denomination.
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
    pub fee_config: Pubkey,
    pub proceeds_vault: Pubkey,
    /// Default pubkey means absent; mandatory for ManagedToAsset.
    pub liquidity_vault: Pubkey,
    pub token_in_decimals: u8,
    pub token_out_decimals: u8,
    pub enabled: bool,
    pub bump: u8,
}
