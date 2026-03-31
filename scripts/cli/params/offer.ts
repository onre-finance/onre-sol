import { ParamDefinition } from "../prompts/types";
import { tokenPairParams } from "./common";
import type { NetworkConfig } from "../../utils/script-helper";

/**
 * Offer command parameter definitions
 */

export const makeOfferParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "fee",
        type: "basisPoints",
        description: "Fee in basis points",
        required: false,
        flag: "--fee",
        shortFlag: "-f",
        default: 0,
    },
    {
        name: "needsApproval",
        type: "boolean",
        description: "Require approval for transactions",
        required: false,
        flag: "--needs-approval",
        default: false,
    },
    {
        name: "permissionless",
        type: "boolean",
        description: "Allow permissionless transactions",
        required: false,
        flag: "--permissionless",
        default: true,
    },
];

export const addVectorParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "baseTime",
        type: "timestamp",
        description: "Base time for the vector",
        required: true,
        flag: "--base-time",
    },
    {
        name: "basePrice",
        type: "amount",
        description: "Base price (scaled by 1e9, so 1.0 = 1000000000)",
        required: true,
        flag: "--base-price",
        default: 1_000_000_000,
    },
    {
        name: "apr",
        type: "apr",
        description: "APR value (scale=6, so 1% = 10000)",
        required: true,
        flag: "--apr",
    },
    {
        name: "duration",
        type: "duration",
        description: "Price fix duration in seconds",
        required: true,
        flag: "--duration",
    },
];

export const deleteVectorParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "startTime",
        type: "timestamp",
        description: "Vector start timestamp to delete",
        required: true,
        flag: "--start-time",
    },
];

export const updateFeeParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "fee",
        type: "basisPoints",
        description: "New fee in basis points",
        required: true,
        flag: "--fee",
        shortFlag: "-f",
    },
];

export const takeOfferParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "amount",
        type: "amount",
        description: "Amount of token in to provide (raw, with decimals)",
        required: true,
        flag: "--amount",
        shortFlag: "-a",
    },
    {
        name: "permissionless",
        type: "boolean",
        description: "Use permissionless flow",
        required: true,
        flag: "--permissionless",
        default: false,
    },
];

export const fetchOfferParams: ParamDefinition[] = [
    {
        name: "tokenIn",
        type: "mint",
        description: "Token in mint (e.g., USDC)",
        required: false,
        flag: "--token-in",
        shortFlag: "-i",
        default: (cfg: NetworkConfig) => cfg.mints.usdc,
    },
    {
        name: "tokenOut",
        type: "mint",
        description: "Token out mint (e.g., ONyc)",
        required: false,
        flag: "--token-out",
        shortFlag: "-o",
        default: (cfg: NetworkConfig) => cfg.mints.onyc,
    },
];
