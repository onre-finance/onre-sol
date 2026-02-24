use anchor_lang::prelude::*;

#[error_code]
pub enum CacheErrorCode {
    #[msg("Invalid ONyc mint for CACHE state")]
    InvalidOnycMint,
    #[msg("No change")]
    NoChange,
    #[msg("Invalid timestamp")]
    InvalidTimestamp,
    #[msg("Program does not have mint authority for this token")]
    NoMintAuthority,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Result does not fit in u64")]
    ResultOverflow,
    #[msg("Target NAV must be greater than zero")]
    InvalidTargetNav,
    #[msg("Asset adjustment amount exceeds total assets")]
    InvalidAssetAdjustmentAmount,
    #[msg("No burn required for provided NAV inputs")]
    NoBurnNeeded,
    #[msg("Burn amount exceeds CACHE balance")]
    InsufficientCacheBalance,
    #[msg("Provided NAV inputs imply minting, not burning")]
    InvalidBurnTarget,
}
