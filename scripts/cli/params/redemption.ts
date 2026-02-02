import type { NetworkConfig } from "../../utils/script-helper";
import { ParamDefinition } from "../prompts/types";
import { tokenPairParams as commonTokenPairParams } from "./common";

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
        default: (cfg: NetworkConfig) => cfg.mints.onyc
    },
    {
        name: "tokenOut",
        type: "mint",
        description: "Token out mint (USDC for redemptions)",
        required: true,
        flag: "--token-out",
        shortFlag: "-o",
        default: (cfg: NetworkConfig) => cfg.mints.usdc
    }
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
        default: 0
    }
];

export const createRequestParams: ParamDefinition[] = [
    ...redemptionTokenPairParams,
    {
        name: "amount",
        type: "amount",
        description: "Amount of tokens to redeem",
        required: true,
        flag: "--amount",
        shortFlag: "-a"
    }
];

export const requestParams: ParamDefinition[] = [
    ...redemptionTokenPairParams,
    {
        name: "requestId",
        type: "string",
        description: "Redemption request ID",
        required: true,
        flag: "--request-id",
        transform: (value: any) => parseInt(value, 10)
    }
];

export const updateRedemptionFeeParams: ParamDefinition[] = [
    ...redemptionTokenPairParams,
    {
        name: "fee",
        type: "basisPoints",
        description: "New redemption fee in basis points",
        required: true,
        flag: "--fee",
        shortFlag: "-f"
    }
];
