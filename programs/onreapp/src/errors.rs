use anchor_lang::prelude::*;

#[error_code]
pub enum OnreError {
    #[msg("Math Overflow")]
    MathOverflow,
    #[msg("Max Supply Exceeded")]
    MaxSupplyExceeded,
    #[msg("Transfer Fee Not Supported")]
    TransferFeeNotSupported,
    #[msg("Zero Price Not Allowed")]
    ZeroPriceNotAllowed,
    #[msg("Decimals Exceed Max")]
    DecimalsExceedMax,
    #[msg("Result Overflow")]
    ResultOverflow,
    #[msg("Expired")]
    Expired,
    #[msg("Wrong Program")]
    WrongProgram,
    #[msg("Wrong User")]
    WrongUser,
    #[msg("Missing Ed25519 Ix")]
    MissingEd25519Ix,
    #[msg("Wrong Ix Program")]
    WrongIxProgram,
    #[msg("Bad Ed25519 Accounts")]
    BadEd25519Accounts,
    #[msg("Malformed Ed25519 Ix")]
    MalformedEd25519Ix,
    #[msg("Multiple Sigs")]
    MultipleSigs,
    #[msg("Wrong Authority")]
    WrongAuthority,
    #[msg("Msg Mismatch")]
    MsgMismatch,
    #[msg("Msg Deserialize")]
    MsgDeserialize,
    #[msg("Invalid Fee")]
    InvalidFee,
    #[msg("Invalid Token In Mint")]
    InvalidTokenInMint,
    #[msg("Invalid Token Out Mint")]
    InvalidTokenOutMint,
    #[msg("Vector Not Found")]
    VectorNotFound,
    #[msg("Start Time In Past")]
    StartTimeInPast,
    #[msg("Invalid Boss")]
    InvalidBoss,
    #[msg("Kill Switch Activated")]
    KillSwitchActivated,
    #[msg("Permissionless Not Allowed")]
    PermissionlessNotAllowed,
    #[msg("Invalid Market Stats Pda")]
    InvalidMarketStatsPda,
    #[msg("Market Stats Not Writable")]
    MarketStatsNotWritable,
    #[msg("Invalid Instructions Sysvar")]
    InvalidInstructionsSysvar,
    #[msg("Invalid Permissionless Token Out Account")]
    InvalidPermissionlessTokenOutAccount,
    #[msg("Invalid User Token Out Account")]
    InvalidUserTokenOutAccount,
    #[msg("Invalid Boss Token In Account")]
    InvalidBossTokenInAccount,
    #[msg("Invalid Time Range")]
    InvalidTimeRange,
    #[msg("Zero Value")]
    ZeroValue,
    #[msg("Duplicate Start Time")]
    DuplicateStartTime,
    #[msg("Too Many Vectors")]
    TooManyVectors,
    #[msg("Invalid A P R")]
    InvalidAPR,
    #[msg("Invalid Price Fix Duration")]
    InvalidPriceFixDuration,
    #[msg("Invalid Vault Authority")]
    InvalidVaultAuthority,
    #[msg("Invalid Mint Authority")]
    InvalidMintAuthority,
    #[msg("Offer Not Found")]
    OfferNotFound,
    #[msg("No Active Vector")]
    NoActiveVector,
    #[msg("Overflow Error")]
    OverflowError,
    #[msg("Approval Required")]
    ApprovalRequired,
    #[msg("Account Full")]
    AccountFull,
    #[msg("Invalid Token Program")]
    InvalidTokenProgram,
    #[msg("Invalid Onyc Mint")]
    InvalidOnycMint,
    #[msg("Invalid Market Stats Owner")]
    InvalidMarketStatsOwner,
    #[msg("Invalid Market Stats Data")]
    InvalidMarketStatsData,
    #[msg("Overflow")]
    Overflow,
    #[msg("Invalid Main Offer")]
    InvalidMainOffer,
    #[msg("Div By Zero")]
    DivByZero,
    #[msg("Invalid Vault Account")]
    InvalidVaultAccount,
    #[msg("Boss Already Set")]
    BossAlreadySet,
    #[msg("Wrong Boss")]
    WrongBoss,
    #[msg("Wrong Owner")]
    WrongOwner,
    #[msg("Immutable Program")]
    ImmutableProgram,
    #[msg("Wrong Program Data")]
    WrongProgramData,
    #[msg("Missing Program Data")]
    MissingProgramData,
    #[msg("Deserialize Program Data Failed")]
    DeserializeProgramDataFailed,
    #[msg("Not Program Data")]
    NotProgramData,
    #[msg("Invalid Permissionless Account Name")]
    InvalidPermissionlessAccountName,
    #[msg("Both Approvers Filled")]
    BothApproversFilled,
    #[msg("Invalid Approver")]
    InvalidApprover,
    #[msg("Approver Already Exists")]
    ApproverAlreadyExists,
    #[msg("Only Boss Can Disable")]
    OnlyBossCanDisable,
    #[msg("Unauthorized To Enable")]
    UnauthorizedToEnable,
    #[msg("Not An Approver")]
    NotAnApprover,
    #[msg("Invalid State Owner")]
    InvalidStateOwner,
    #[msg("Invalid State Pda")]
    InvalidStatePda,
    #[msg("Invalid State Data")]
    InvalidStateData,
    #[msg("Unauthorized Signer")]
    UnauthorizedSigner,
    #[msg("Lamport Overflow")]
    LamportOverflow,
    #[msg("No Boss Proposal")]
    NoBossProposal,
    #[msg("Not Proposed Boss")]
    NotProposedBoss,
    #[msg("Invalid Boss Address")]
    InvalidBossAddress,
    #[msg("No Change")]
    NoChange,
    #[msg("Admin Already Exists")]
    AdminAlreadyExists,
    #[msg("Max Admins Reached")]
    MaxAdminsReached,
    #[msg("Admin Not Found")]
    AdminNotFound,
    #[msg("Program Not Mint Authority")]
    ProgramNotMintAuthority,
    #[msg("No Mint Authority")]
    NoMintAuthority,
    #[msg("Boss Not Mint Authority")]
    BossNotMintAuthority,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Zero Balance")]
    ZeroBalance,
    #[msg("Insufficient Balance")]
    InsufficientBalance,
    #[msg("Arithmetic Overflow")]
    ArithmeticOverflow,
    #[msg("Invalid Mint")]
    InvalidMint,
    #[msg("Invalid Redemption Offer")]
    InvalidRedemptionOffer,
    #[msg("Arithmetic Underflow")]
    ArithmeticUnderflow,
    #[msg("Invalid Redeemer")]
    InvalidRedeemer,
    #[msg("Invalid Redemption Admin")]
    InvalidRedemptionAdmin,
    #[msg("Invalid Redeemer Token Account")]
    InvalidRedeemerTokenAccount,
    #[msg("Offer Mismatch")]
    OfferMismatch,
    #[msg("Offer Mint Mismatch")]
    OfferMintMismatch,
    #[msg("Invalid Redemption Offer Owner")]
    InvalidRedemptionOfferOwner,
    #[msg("Invalid Redemption Offer Data")]
    InvalidRedemptionOfferData,
    #[msg("Invalid Redemption Fee Vault Authority")]
    InvalidRedemptionFeeVaultAuthority,
    #[msg("Invalid Redemption Fee Vault Authority Owner")]
    InvalidRedemptionFeeVaultAuthorityOwner,
    #[msg("Invalid Redemption Fee Vault Authority Data")]
    InvalidRedemptionFeeVaultAuthorityData,
    #[msg("Invalid Fee Destination Token In Account")]
    InvalidFeeDestinationTokenInAccount,
    #[msg("Invalid Offer Vault Onyc Account")]
    InvalidOfferVaultOnycAccount,
    #[msg("Invalid Vault Token In Account")]
    InvalidVaultTokenInAccount,
    #[msg("Invalid Vault Token Out Account")]
    InvalidVaultTokenOutAccount,
    #[msg("Invalid Amount")]
    InvalidAmount,
    #[msg("Amount Exceeds Remaining")]
    AmountExceedsRemaining,
    #[msg("Invalid Fee Destination")]
    InvalidFeeDestination,
    #[msg("Invalid Buffer State Account")]
    InvalidBufferStateAccount,
    #[msg("Invalid Timestamp")]
    InvalidTimestamp,
    #[msg("Minimum Out Not Met")]
    MinimumOutNotMet,
    #[msg("Invalid Swap Pair")]
    InvalidSwapPair,
    #[msg("Invalid Fee Wallet")]
    InvalidFeeWallet,
    #[msg("Invalid Target Nav")]
    InvalidTargetNav,
    #[msg("Invalid Asset Adjustment Amount")]
    InvalidAssetAdjustmentAmount,
    #[msg("No Burn Needed")]
    NoBurnNeeded,
    #[msg("Insufficient Cache Balance")]
    InsufficientCacheBalance,
    #[msg("Insufficient Fee Balance")]
    InsufficientFeeBalance,
    #[msg("Invalid Fee Recipient")]
    InvalidFeeRecipient,
    #[msg("Invalid Burn Target")]
    InvalidBurnTarget,
}
