/**
 * CLI Command Implementations
 *
 * This module contains the actual business logic for CLI commands,
 * separated from command definitions for better organization.
 */

// Vault implementations
export { executeVaultDeposit } from "./vault-deposit";
export { executeVaultWithdraw } from "./vault-withdraw";
export { executeVaultRedemptionDeposit } from "./vault-redemption-deposit";
export { executeVaultRedemptionWithdraw } from "./vault-redemption-withdraw";

// Init implementations
export { executeInitProgram } from "./init-program";
export { executeInitPermissionless } from "./init-permissionless";

// Mint authority implementations
export { executeMintTo } from "./mint-to";
export { executeMintAuthorityToProgram } from "./mint-authority-to-program";
export { executeMintAuthorityToBoss } from "./mint-authority-to-boss";

// Market implementations
export { executeMarketNav } from "./market-nav";
export { executeMarketNavAdjustment } from "./market-nav-adjustment";
export { executeMarketApy } from "./market-apy";
export { executeMarketTvl } from "./market-tvl";
export { executeMarketSupply } from "./market-supply";

// Offer implementations
export { executeOfferMake } from "./offer-make";
export { executeOfferFetch } from "./offer-fetch";
export { executeOfferAddVector } from "./offer-add-vector";
export { executeOfferDeleteVector } from "./offer-delete-vector";
export { executeOfferUpdateFee } from "./offer-update-fee";
export { executeOfferDeleteAllVectors } from "./offer-delete-all-vectors";

// Redemption implementations
export { executeRedemptionMakeOffer } from "./redemption-make-offer";
export { executeRedemptionFetchOffer } from "./redemption-fetch-offer";
export { executeRedemptionUpdateFee } from "./redemption-update-fee";
export { executeRedemptionCreateRequest } from "./redemption-create-request";
export { executeRedemptionFetchRequest } from "./redemption-fetch-request";
export { executeRedemptionFulfill } from "./redemption-fulfill";
export { executeRedemptionCancel } from "./redemption-cancel";
export { executeRedemptionListRequests } from "./redemption-list-requests";

// State implementations
export { executeStateGet } from "./state-get";
export { executeStateProposeBoss } from "./state-propose-boss";
export { executeStateAcceptBoss } from "./state-accept-boss";
export { executeStateAddAdmin } from "./state-add-admin";
export { executeStateRemoveAdmin } from "./state-remove-admin";
export { executeStateAddApprover } from "./state-add-approver";
export { executeStateRemoveApprover } from "./state-remove-approver";
export { executeStateSetOnycMint } from "./state-set-onyc-mint";
export { executeStateKillSwitch } from "./state-kill-switch";
export { executeStateMaxSupply } from "./state-max-supply";
export { executeStateSetRedemptionAdmin } from "./state-set-redemption-admin";
export { executeStateClearAdmins } from "./state-clear-admins";
export { executeStateClose } from "./state-close";

// Market implementations
// export { executeMarketNav } from "./market-nav";
// ... etc
