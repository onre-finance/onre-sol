import type { NetworkConfig } from "../../utils/script-helper";

/**
 * Parameter types supported by the CLI prompting system
 */
export type ParamType =
    | "publicKey" // Solana PublicKey
    | "mint" // Mint selection (usdc, onyc, usdg, or custom)
    | "amount" // Token amount (raw integer)
    | "basisPoints" // Fee in basis points (0-10000)
    | "apr" // APR value (scaled by 1_000_000)
    | "timestamp" // Unix timestamp or date string
    | "duration" // Duration in seconds
    | "boolean" // True/false
    | "string" // Generic string
    | "select"; // Select from options

/**
 * Definition for a command parameter
 */
export interface ParamDefinition {
    /** Internal parameter name */
    name: string;
    /** Parameter type for validation and prompting */
    type: ParamType;
    /** Human-readable description shown in prompts */
    description: string;
    /** Whether the parameter is required */
    required: boolean;
    /** Default value (can be a function that receives config) */
    default?: any | ((config: NetworkConfig) => any);
    /** CLI flag (e.g., "--token-in") */
    flag?: string;
    /** Short CLI flag (e.g., "-i") */
    shortFlag?: string;
    /** Choices for select type */
    choices?: Array<{ name: string; value: string }>;
    /** Custom validation function */
    validate?: (value: any) => boolean | string;
    /** Transform function applied after input */
    transform?: (value: any, config: NetworkConfig) => any;
}

/**
 * Result of parameter prompting
 */
export interface PromptResult {
    [key: string]: any;
}

/**
 * Global CLI options passed through commands
 */
export interface GlobalOptions {
    network?: string;
    json?: boolean;
    dryRun?: boolean;
    noInteractive?: boolean;
}
