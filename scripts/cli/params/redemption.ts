import type { NetworkConfig } from "../../utils/script-helper";
import { ParamDefinition } from "../prompts/types";
import { PublicKey } from "@solana/web3.js";

/**
 * Redemption command parameter definitions
 */

// For redemptions, default is ONyc -> USDC (different from offer's USDC -> ONyc)
const redemptionTokenPairParams: ParamDefinition[] = [
    {
        name: "tokenIn",
        type: "mint",
        description: "Token in mint (ONyc for redemptions)",
        required: true,
        flag: "--token-in",
        shortFlag: "-i",
        default: (cfg: NetworkConfig) => cfg.mints.onyc,
    },
    {
        name: "tokenOut",
        type: "mint",
        description: "Token out mint (USDC or USDT for redemptions)",
        required: true,
        flag: "--token-out",
        shortFlag: "-o",
        default: (cfg: NetworkConfig) => cfg.mints.usdc,
    },
];

export { redemptionTokenPairParams as tokenPairParams };

export const redemptionOfferParams: ParamDefinition[] = [
    ...redemptionTokenPairParams,
    {
        name: "fee",
        type: "basisPoints",
        description: "Redemption fee in basis points",
        required: false,
        flag: "--fee",
        shortFlag: "-f",
        default: 0,
    },
    {
        name: "propAmmSellFee",
        type: "basisPoints",
        description: "Prop AMM sell redemption fee in basis points",
        required: false,
        flag: "--prop-amm-sell-fee",
        default: 0,
    },
];

export const createRequestParams: ParamDefinition[] = [
    ...redemptionTokenPairParams,
    {
        name: "requestId",
        type: "string",
        description: "Frontend-generated random request ID (exactly 32 UTF-8 bytes; UUID without hyphens)",
        required: true,
        flag: "--request-id",
        validate: validateRedemptionRequestId,
    },
    {
        name: "amount",
        type: "amount",
        description: "Amount of tokens to redeem",
        required: true,
        flag: "--amount",
        shortFlag: "-a",
    },
];

export const requestParams: ParamDefinition[] = [
    ...redemptionTokenPairParams,
    {
        name: "requestId",
        type: "string",
        description: "Frontend-generated 32-byte redemption request ID",
        required: true,
        flag: "--request-id",
        validate: validateRedemptionRequestId,
    },
    {
        name: "redeemer",
        type: "string",
        description: "Redeemer public key used in request PDA derivation",
        required: true,
        flag: "--redeemer",
        validate: (value: string) => {
            try {
                new PublicKey(value.trim());
                return true;
            } catch {
                return "Invalid public key format";
            }
        },
        transform: (value: string) => new PublicKey(value.trim()),
    },
];

function validateRedemptionRequestId(value: string): true | string {
    const length = Buffer.byteLength(value, "utf8");
    return length === 32
        ? true
        : "Request ID must be exactly 32 UTF-8 bytes (use a UUID without hyphens)";
}

export const fulfillRequestParams: ParamDefinition[] = [
    ...requestParams,
    {
        name: "amount",
        type: "amount",
        description: "Amount of token_in to fulfill in this call (leave empty to fulfill all remaining)",
        required: false,
        flag: "--amount",
        shortFlag: "-a",
    },
];

export const updateRedemptionFeeParams: ParamDefinition[] = [
    ...redemptionTokenPairParams,
    {
        name: "fee",
        type: "basisPoints",
        description: "New redemption fee in basis points",
        required: true,
        flag: "--fee",
        shortFlag: "-f",
    },
];

export const setRedemptionDisabledParams: ParamDefinition[] = [
    ...redemptionTokenPairParams,
    {
        name: "disabled",
        type: "boolean",
        description: "Whether the redemption offer should be disabled",
        required: true,
        flag: "--disabled",
    },
];

export const updateRedemptionVaultTargetParams: ParamDefinition[] = [
    ...redemptionTokenPairParams,
    {
        name: "vaultTargetBps",
        type: "string",
        description: "New redemption vault target in basis points",
        required: true,
        flag: "--target-bps",
        transform: (value: any) => parseInt(value, 10),
    },
];

export const listRequestsParams: ParamDefinition[] = [
    ...redemptionTokenPairParams,
    {
        name: "redeemer",
        type: "string",
        description: "Filter by redeemer address (leave empty for all)",
        required: false,
        flag: "--redeemer",
        shortFlag: "-r",
        validate: (value: string) => {
            // Allow empty values for optional parameter
            if (!value || value.trim() === "") {
                return true;
            }
            try {
                new PublicKey(value.trim());
                return true;
            } catch {
                return "Invalid public key format";
            }
        },
        transform: (value: any) => {
            // Return undefined for empty values
            if (!value || (typeof value === "string" && value.trim() === "")) {
                return undefined;
            }
            return typeof value === "string" ? new PublicKey(value.trim()) : value;
        },
    },
];
