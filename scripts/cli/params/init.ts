import type { NetworkConfig } from "../../utils/script-helper";
import { ParamDefinition } from "../prompts/types";

/**
 * Init command parameter definitions
 */

export const initProgramParams: ParamDefinition[] = [
    {
        name: "onycMint",
        type: "mint",
        description: "ONyc mint address",
        required: true,
        flag: "--onyc-mint",
        default: (cfg: NetworkConfig) => cfg.mints.onyc,
    },
];

export const initPermissionlessParams: ParamDefinition[] = [
    {
        name: "name",
        type: "string",
        description: "Authority name (e.g., 'permissionless-1')",
        required: true,
        flag: "--name",
        default: "permissionless-1",
    },
];
