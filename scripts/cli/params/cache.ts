import type { NetworkConfig } from "../../utils/script-helper";
import { ParamDefinition } from "../prompts/types";

export const cacheInitParams: ParamDefinition[] = [
    {
        name: "cacheAdmin",
        type: "publicKey",
        description: "CACHE admin public key",
        required: true,
        flag: "--cache-admin",
    },
    {
        name: "onycMint",
        type: "mint",
        description: "ONyc mint address",
        required: true,
        flag: "--onyc-mint",
        default: (cfg: NetworkConfig) => cfg.mints.onyc,
    },
];

export const cacheOnycMintParam: ParamDefinition[] = [
    {
        name: "onycMint",
        type: "mint",
        description: "ONyc mint address",
        required: true,
        flag: "--onyc-mint",
        default: (cfg: NetworkConfig) => cfg.mints.onyc,
    },
];

export const cacheAdminParam: ParamDefinition[] = [
    {
        name: "cacheAdmin",
        type: "publicKey",
        description: "CACHE admin public key",
        required: true,
        flag: "--cache-admin",
    },
];

export const cacheYieldsParams: ParamDefinition[] = [
    {
        name: "grossYield",
        type: "apr",
        description: "Gross yield (scale=1e6; 100000 = 10%)",
        required: true,
        flag: "--gross-yield",
    },
    {
        name: "currentYield",
        type: "apr",
        description: "Current distributed yield (scale=1e6; 100000 = 10%)",
        required: true,
        flag: "--current-yield",
    },
];

export const cacheBurnParams: ParamDefinition[] = [
    {
        name: "tokenIn",
        type: "mint",
        description: "Offer token-in mint used for NAV/TVL context (e.g. USDC)",
        required: true,
        flag: "--token-in",
        default: (cfg: NetworkConfig) => cfg.mints.usdc,
    },
    {
        name: "assetAdjustmentAmount",
        type: "amount",
        description: "Asset adjustment amount used by burn formula (raw, 9 decimals)",
        required: true,
        flag: "--asset-adjustment-amount",
    },
    {
        name: "targetNav",
        type: "amount",
        description: "Target NAV value (raw, 9 decimals)",
        required: true,
        flag: "--target-nav",
    },
    {
        name: "onycMint",
        type: "mint",
        description: "ONyc mint address",
        required: true,
        flag: "--onyc-mint",
        default: (cfg: NetworkConfig) => cfg.mints.onyc,
    },
];
