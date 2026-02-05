import type { NetworkConfig } from "../../utils/script-helper";
import { ParamDefinition } from "../prompts/types";

/**
 * Mint authority command parameter definitions
 */

export const mintParams: ParamDefinition[] = [
    {
        name: "mint",
        type: "mint",
        description: "Mint address",
        required: true,
        flag: "--mint",
        shortFlag: "-m",
        default: (cfg: NetworkConfig) => cfg.mints.onyc,
    },
    {
        name: "amount",
        type: "amount",
        description: "Amount to mint",
        required: true,
        flag: "--amount",
        shortFlag: "-a",
    },
];

export const transferMintAuthorityParams: ParamDefinition[] = [
    {
        name: "mint",
        type: "mint",
        description: "Mint address",
        required: true,
        flag: "--mint",
        shortFlag: "-m",
        default: (cfg: NetworkConfig) => cfg.mints.onyc,
    },
];
