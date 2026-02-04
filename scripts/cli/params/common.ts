import type { NetworkConfig } from "../../utils/script-helper";
import { ParamDefinition } from "../prompts/types";

/**
 * Common parameter definitions shared across multiple commands
 *
 * These parameters are used by multiple command domains to avoid duplication.
 */

/**
 * Token pair parameters (token in + token out)
 * Used by: offer, redemption, market commands
 */
export const tokenPairParams: ParamDefinition[] = [
    {
        name: "tokenIn",
        type: "mint",
        description: "Token in mint (e.g., USDC)",
        required: true,
        flag: "--token-in",
        shortFlag: "-i",
        default: (cfg: NetworkConfig) => cfg.mints.usdc,
    },
    {
        name: "tokenOut",
        type: "mint",
        description: "Token out mint (e.g., ONyc)",
        required: true,
        flag: "--token-out",
        shortFlag: "-o",
        default: (cfg: NetworkConfig) => cfg.mints.onyc,
    },
];

/**
 * Vault parameters (token mint + amount)
 * Used by: vault deposit/withdraw commands
 */
export const vaultParams: ParamDefinition[] = [
    {
        name: "tokenMint",
        type: "mint",
        description: "Token mint to deposit/withdraw",
        required: true,
        flag: "--token",
        shortFlag: "-t",
    },
    {
        name: "amount",
        type: "amount",
        description: "Amount (raw, with decimals)",
        required: true,
        flag: "--amount",
        shortFlag: "-a",
    },
];

/**
 * Fee parameter (basis points)
 * Used by: offer, redemption commands
 */
export const feeParam: ParamDefinition = {
    name: "fee",
    type: "basisPoints",
    description: "Fee in basis points (100 = 1%)",
    required: true,
    flag: "--fee",
    shortFlag: "-f",
};
