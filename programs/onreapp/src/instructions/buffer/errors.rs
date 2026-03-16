use anchor_lang::prelude::*;

#[error_code]
pub enum BufferErrorCode {
    #[msg("Invalid ONyc mint for BUFFER state")]
    InvalidOnycMint,
    #[msg("Invalid main offer for BUFFER state")]
    InvalidMainOffer,
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
    #[msg("Invalid fee: basis points must be <= 10000")]
    InvalidFee,
    #[msg("Fee wallet must be set when the fee is enabled")]
    InvalidFeeWallet,
    #[msg("Target NAV must be greater than zero")]
    InvalidTargetNav,
    #[msg("Asset adjustment amount exceeds total assets")]
    InvalidAssetAdjustmentAmount,
    #[msg("No burn required for provided NAV inputs")]
    NoBurnNeeded,
    #[msg("Burn amount exceeds BUFFER balance")]
    InsufficientCacheBalance,
    #[msg("Fee amount exceeds fee vault balance")]
    InsufficientFeeBalance,
    #[msg("Fee recipient does not match configured fee wallet")]
    InvalidFeeRecipient,
    #[msg("Provided NAV inputs imply minting, not burning")]
    InvalidBurnTarget,
}
