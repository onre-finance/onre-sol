import { confirm, input, number, select } from "@inquirer/prompts";
import { PublicKey } from "@solana/web3.js";
import chalk from "chalk";
import type { NetworkConfig } from "../../utils/script-helper";
import type { ParamDefinition, PromptResult } from "./types";
import {
    parseTimestamp,
    validateAmount,
    validateApr,
    validateBasisPoints,
    validateDuration,
    validatePublicKey,
    validateTimestamp
} from "./validators";

/**
 * Prompt for all missing parameters in a command
 */
export async function promptForParams(
    params: ParamDefinition[],
    providedValues: Record<string, any>,
    config: NetworkConfig,
    noInteractive: boolean = false
): Promise<PromptResult> {
    const result: PromptResult = {};

    for (const param of params) {
        // Check if value was provided via CLI
        if (providedValues[param.name] !== undefined) {
            result[param.name] = transformValue(providedValues[param.name], param, config);
            continue;
        }

        // Check for default value
        if (param.default !== undefined) {
            const defaultValue = typeof param.default === "function"
                ? param.default(config)
                : param.default;

            if (!param.required || noInteractive) {
                result[param.name] = defaultValue;
                continue;
            }
        }

        // If non-interactive and required with no default, error
        if (noInteractive && param.required) {
            throw new Error(`Missing required parameter: ${param.name} (${param.flag || param.name})`);
        }

        // Prompt for the value
        result[param.name] = await promptForSingleParam(param, config);
    }

    return result;
}

/**
 * Prompt for a single parameter based on its type
 */
async function promptForSingleParam(
    param: ParamDefinition,
    config: NetworkConfig
): Promise<any> {
    const message = param.description;
    const defaultValue = typeof param.default === "function"
        ? param.default(config)
        : param.default;

    switch (param.type) {
        case "publicKey":
            return promptPublicKey(message, defaultValue);

        case "mint":
            return promptMint(message, config, defaultValue);

        case "amount":
            return promptAmount(message, defaultValue);

        case "basisPoints":
            return promptBasisPoints(message, defaultValue);

        case "apr":
            return promptApr(message, defaultValue);

        case "timestamp":
            return promptTimestamp(message, defaultValue);

        case "duration":
            return promptDuration(message, defaultValue);

        case "boolean":
            return confirm({ message, default: defaultValue ?? false });

        case "select":
            return select({
                message,
                choices: param.choices || [],
                default: defaultValue
            });

        case "string":
        default:
            return input({
                message,
                default: defaultValue?.toString()
            });
    }
}

/**
 * Prompt for a public key
 */
async function promptPublicKey(message: string, defaultValue?: PublicKey): Promise<PublicKey> {
    const value = await input({
        message,
        default: defaultValue?.toBase58(),
        validate: validatePublicKey
    });
    return new PublicKey(value.trim());
}

/**
 * Prompt for a mint selection
 */
async function promptMint(
    message: string,
    config: NetworkConfig,
    defaultValue?: PublicKey
): Promise<PublicKey> {
    // Determine default selection
    let defaultChoice = "custom";
    if (defaultValue) {
        if (defaultValue.equals(config.mints.usdc)) defaultChoice = "usdc";
        else if (defaultValue.equals(config.mints.onyc)) defaultChoice = "onyc";
        else if (defaultValue.equals(config.mints.usdg)) defaultChoice = "usdg";
    }

    const choices = [
        {
            name: `USDC  ${chalk.gray(config.mints.usdc.toBase58().slice(0, 8) + "...")}`,
            value: "usdc"
        },
        {
            name: `ONyc  ${chalk.gray(config.mints.onyc.toBase58().slice(0, 8) + "...")}`,
            value: "onyc"
        },
        {
            name: `USDG  ${chalk.gray(config.mints.usdg.toBase58().slice(0, 8) + "...")}`,
            value: "usdg"
        },
        {
            name: "Custom address",
            value: "custom"
        }
    ];

    const selection = await select({
        message,
        choices,
        default: defaultChoice
    });

    if (selection === "custom") {
        const address = await input({
            message: "Enter mint address:",
            default: defaultValue?.toBase58(),
            validate: validatePublicKey
        });
        return new PublicKey(address.trim());
    }

    return config.mints[selection as keyof typeof config.mints];
}

/**
 * Prompt for a token amount (raw integer)
 */
async function promptAmount(message: string, defaultValue?: number): Promise<number> {
    const value = await number({
        message,
        default: defaultValue,
        validate: (val) => {
            if (val === undefined) return "Amount is required";
            return validateAmount(val);
        }
    });
    return value!;
}

/**
 * Prompt for basis points
 */
async function promptBasisPoints(message: string, defaultValue?: number): Promise<number> {
    const value = await number({
        message: `${message} (100 = 1%)`,
        default: defaultValue ?? 0,
        validate: (val) => {
            if (val === undefined) return true; // Allow empty for default
            return validateBasisPoints(val);
        }
    });
    return value ?? defaultValue ?? 0;
}

/**
 * Prompt for APR value
 */
async function promptApr(message: string, defaultValue?: number): Promise<number> {
    const value = await number({
        message: `${message} (e.g., 36500 = 3.65%)`,
        default: defaultValue,
        validate: (val) => {
            if (val === undefined) return "APR is required";
            return validateApr(val);
        }
    });
    return value!;
}

/**
 * Prompt for a timestamp
 */
async function promptTimestamp(message: string, defaultValue?: number | string): Promise<number> {
    const defaultStr = defaultValue !== undefined
        ? (typeof defaultValue === "number"
            ? new Date(defaultValue * 1000).toISOString()
            : defaultValue)
        : "now";

    const value = await input({
        message: `${message} (ISO date or "now")`,
        default: defaultStr,
        validate: validateTimestamp
    });

    return parseTimestamp(value);
}

/**
 * Prompt for duration in seconds
 */
async function promptDuration(message: string, defaultValue?: number): Promise<number> {
    const value = await number({
        message: `${message} (in seconds)`,
        default: defaultValue,
        validate: (val) => {
            if (val === undefined) return "Duration is required";
            return validateDuration(val);
        }
    });
    return value!;
}

/**
 * Transform a CLI-provided value to the appropriate type
 */
function transformValue(
    value: any,
    param: ParamDefinition,
    config: NetworkConfig
): any {
    // Apply custom transform if provided
    if (param.transform) {
        return param.transform(value, config);
    }

    switch (param.type) {
        case "publicKey":
            return typeof value === "string" ? new PublicKey(value) : value;

        case "mint":
            if (typeof value === "string") {
                // Check for known mint aliases
                const lower = value.toLowerCase();
                if (lower === "usdc") return config.mints.usdc;
                if (lower === "onyc") return config.mints.onyc;
                if (lower === "usdg") return config.mints.usdg;
                return new PublicKey(value);
            }
            return value;

        case "timestamp":
            return parseTimestamp(value);

        case "amount":
        case "basisPoints":
        case "apr":
        case "duration":
            return typeof value === "string" ? parseInt(value, 10) : value;

        case "boolean":
            if (typeof value === "string") {
                return value.toLowerCase() === "true" || value === "1";
            }
            return !!value;

        default:
            return value;
    }
}

export * from "./types";
export * from "./validators";
