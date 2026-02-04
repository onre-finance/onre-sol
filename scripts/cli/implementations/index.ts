/**
 * CLI Command Implementations
 *
 * This module contains the actual business logic for CLI commands,
 * separated from command definitions for better organization.
 */

// Vault implementations
export { executeVaultDeposit } from "./vault/vault-deposit";
export { executeVaultWithdraw } from "./vault/vault-withdraw";
export { executeVaultRedemptionDeposit } from "./vault/vault-redemption-deposit";
export { executeVaultRedemptionWithdraw } from "./vault/vault-redemption-withdraw";

// Init implementations
export { executeInitProgram } from "./init/init-program";
export { executeInitPermissionless } from "./init/init-permissionless";

// Mint authority implementations
export { executeMintTo } from "./mint-authority/mint-to";
export { executeMintAuthorityToProgram } from "./mint-authority/mint-authority-to-program";
export { executeMintAuthorityToBoss } from "./mint-authority/mint-authority-to-boss";

// Market implementations
export { executeMarketNav } from "./market/market-nav";
export { executeMarketNavAdjustment } from "./market/market-nav-adjustment";
export { executeMarketApy } from "./market/market-apy";
export { executeMarketTvl } from "./market/market-tvl";
export { executeMarketSupply } from "./market/market-supply";

// Offer implementations
export { executeOfferMake } from "./offer/offer-make";
export { executeOfferFetch } from "./offer/offer-fetch";
export { executeOfferTake } from "./offer/offer-take";
export { executeOfferTakePermissionless } from "./offer/offer-take-permissionless";
export { executeOfferAddVector } from "./offer/offer-add-vector";
export { executeOfferDeleteVector } from "./offer/offer-delete-vector";
export { executeOfferUpdateFee } from "./offer/offer-update-fee";
export { executeOfferDeleteAllVectors } from "./offer/offer-delete-all-vectors";

// Redemption implementations
export { executeRedemptionMakeOffer } from "./redemption/redemption-make-offer";
export { executeRedemptionFetchOffer } from "./redemption/redemption-fetch-offer";
export { executeRedemptionUpdateFee } from "./redemption/redemption-update-fee";
export { executeRedemptionCreateRequest } from "./redemption/redemption-create-request";
export { executeRedemptionFetchRequest } from "./redemption/redemption-fetch-request";
export { executeRedemptionFulfill } from "./redemption/redemption-fulfill";
export { executeRedemptionCancel } from "./redemption/redemption-cancel";
export { executeRedemptionListRequests } from "./redemption/redemption-list-requests";

// State implementations
export { executeStateGet } from "./state/state-get";
export { executeStateProposeBoss } from "./state/state-propose-boss";
export { executeStateAcceptBoss } from "./state/state-accept-boss";
export { executeStateAddAdmin } from "./state/state-add-admin";
export { executeStateRemoveAdmin } from "./state/state-remove-admin";
export { executeStateAddApprover } from "./state/state-add-approver";
export { executeStateRemoveApprover } from "./state/state-remove-approver";
export { executeStateSetOnycMint } from "./state/state-set-onyc-mint";
export { executeStateKillSwitch } from "./state/state-kill-switch";
export { executeStateMaxSupply } from "./state/state-max-supply";
export { executeStateSetRedemptionAdmin } from "./state/state-set-redemption-admin";
export { executeStateClearAdmins } from "./state/state-clear-admins";
export { executeStateClose } from "./state/state-close";
