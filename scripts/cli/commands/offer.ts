import { Command } from "commander";
import chalk from "chalk";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { config, NetworkConfig, ScriptHelper } from "../../utils/script-helper";
import type { GlobalOptions } from "../prompts";
import { ParamDefinition, promptForParams } from "../prompts";
import { handleTransaction } from "../transaction/handler";
import { printNetworkBanner, printOffer, printParamSummary } from "../utils/display";

// Common token pair params
// Note: param names match Commander's camelCase conversion (--token-in -> tokenIn)
const tokenPairParams: ParamDefinition[] = [
    {
        name: "tokenIn",
        type: "mint",
        description: "Token in mint (e.g., USDC)",
        required: true,
        flag: "--token-in",
        shortFlag: "-i",
        default: (cfg: NetworkConfig) => cfg.mints.usdc
    },
    {
        name: "tokenOut",
        type: "mint",
        description: "Token out mint (e.g., ONyc)",
        required: true,
        flag: "--token-out",
        shortFlag: "-o",
        default: (cfg: NetworkConfig) => cfg.mints.onyc
    }
];

/**
 * Register offer subcommands
 */
export function registerOfferCommands(program: Command): void {
    // offer make
    program
        .command("make")
        .description("Create a new token offer")
        .option("-i, --token-in <mint>", "Token in mint")
        .option("-o, --token-out <mint>", "Token out mint")
        .option("-f, --fee <bps>", "Fee in basis points (100 = 1%)")
        .option("--needs-approval", "Require approval for transactions")
        .option("--permissionless", "Allow permissionless transactions")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeMakeOffer(opts);
        });

    // offer fetch
    program
        .command("fetch")
        .description("Fetch and display offer details")
        .option("-i, --token-in <mint>", "Token in mint")
        .option("-o, --token-out <mint>", "Token out mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeFetchOffer(opts);
        });

    // offer add-vector
    program
        .command("add-vector")
        .description("Add a pricing vector to an offer")
        .option("-i, --token-in <mint>", "Token in mint")
        .option("-o, --token-out <mint>", "Token out mint")
        .option("--base-time <timestamp>", "Base time (ISO date or unix timestamp)")
        .option("--base-price <price>", "Base price (scaled by 1e9)")
        .option("--apr <value>", "APR in basis points (36500 = 3.65%)")
        .option("--duration <seconds>", "Price fix duration in seconds")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeAddVector(opts);
        });

    // offer delete-vector
    program
        .command("delete-vector")
        .description("Delete a pricing vector from an offer")
        .option("-i, --token-in <mint>", "Token in mint")
        .option("-o, --token-out <mint>", "Token out mint")
        .option("--start-time <timestamp>", "Vector start timestamp to delete")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeDeleteVector(opts);
        });

    // offer update-fee
    program
        .command("update-fee")
        .description("Update the fee for an offer")
        .option("-i, --token-in <mint>", "Token in mint")
        .option("-o, --token-out <mint>", "Token out mint")
        .option("-f, --fee <bps>", "New fee in basis points")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeUpdateFee(opts);
        });

    // offer close
    program
        .command("close")
        .description("Close an offer")
        .option("-i, --token-in <mint>", "Token in mint")
        .option("-o, --token-out <mint>", "Token out mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeCloseOffer(opts);
        });
}

// === Command Implementations ===

const makeOfferParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "fee",
        type: "basisPoints",
        description: "Fee in basis points",
        required: false,
        flag: "--fee",
        shortFlag: "-f",
        default: 0
    },
    {
        name: "needsApproval",
        type: "boolean",
        description: "Require approval for transactions",
        required: false,
        flag: "--needs-approval",
        default: false
    },
    {
        name: "permissionless",
        type: "boolean",
        description: "Allow permissionless transactions",
        required: false,
        flag: "--permissionless",
        default: true
    }
];

async function executeMakeOffer(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(makeOfferParams, opts, config, opts.noInteractive);

        printParamSummary("Creating offer:", {
            tokenIn: params.tokenIn,
            tokenOut: params.tokenOut,
            fee: `${params.fee / 100}% (${params.fee} bps)`,
            needsApproval: params.needsApproval,
            permissionless: params.permissionless
        });

        const boss = await helper.getBoss();
        const instructions = [];

        // Add permissionless account creation if needed
        if (params.permissionless) {
            const permissionlessIxs = await helper.buildCreatePermissionlessTokenAccountsIxs({
                tokenInMint: params.tokenIn,
                tokenOutMint: params.tokenOut,
                tokenInProgram: TOKEN_PROGRAM_ID,
                tokenOutProgram: TOKEN_PROGRAM_ID,
                payer: boss
            });
            instructions.push(...permissionlessIxs);
        }

        // Add main instruction
        const makeOfferIx = await helper.buildMakeOfferIx({
            tokenInMint: params.tokenIn,
            tokenOutMint: params.tokenOut,
            feeBasisPoints: params.fee,
            needsApproval: params.needsApproval,
            allowPermissionless: params.permissionless,
            boss
        });
        instructions.push(makeOfferIx);

        const tx = await helper.prepareTransactionMultipleIxs({ ixs: instructions, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Make Offer Transaction",
            description: `Creates a new ${params.tokenIn.toBase58().slice(0, 8)}... → ${params.tokenOut.toBase58().slice(0, 8)}... offer`,
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeFetchOffer(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(tokenPairParams, opts, config, opts.noInteractive);

        const offer = await helper.getOffer(params.tokenIn, params.tokenOut);

        printOffer(
            offer,
            params.tokenIn.toBase58(),
            params.tokenOut.toBase58(),
            opts.json
        );
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

const addVectorParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "baseTime",
        type: "timestamp",
        description: "Base time for the vector",
        required: true,
        flag: "--base-time"
    },
    {
        name: "basePrice",
        type: "amount",
        description: "Base price (scaled by 1e9, so 1.0 = 1000000000)",
        required: true,
        flag: "--base-price",
        default: 1_000_000_000
    },
    {
        name: "apr",
        type: "apr",
        description: "APR in basis points (36500 = 3.65%)",
        required: true,
        flag: "--apr"
    },
    {
        name: "duration",
        type: "duration",
        description: "Price fix duration",
        required: true,
        flag: "--duration",
        default: 86400 // 1 day
    }
];

async function executeAddVector(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(addVectorParams, opts, config, opts.noInteractive);

        printParamSummary("Adding vector:", {
            tokenIn: params.tokenIn,
            tokenOut: params.tokenOut,
            baseTime: new Date(params.baseTime * 1000).toISOString(),
            basePrice: `${(params.basePrice / 1_000_000_000).toFixed(9)}`,
            apr: `${(params.apr / 10000).toFixed(2)}%`,
            duration: `${params.duration}s`
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildAddOfferVectorIx({
            tokenInMint: params.tokenIn,
            tokenOutMint: params.tokenOut,
            baseTime: params.baseTime,
            basePrice: params.basePrice,
            apr: params.apr,
            priceFixDuration: params.duration,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Add Offer Vector Transaction",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

const deleteVectorParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "startTime",
        type: "timestamp",
        description: "Vector start timestamp to delete",
        required: true,
        flag: "--start-time"
    }
];

async function executeDeleteVector(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(deleteVectorParams, opts, config, opts.noInteractive);

        printParamSummary("Deleting vector:", {
            tokenIn: params.tokenIn,
            tokenOut: params.tokenOut,
            startTime: new Date(params.startTime * 1000).toISOString()
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildDeleteOfferVectorIx({
            tokenInMint: params.tokenIn,
            tokenOutMint: params.tokenOut,
            vectorStartTimestamp: params.startTime,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Delete Offer Vector Transaction",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

const updateFeeParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "fee",
        type: "basisPoints",
        description: "New fee in basis points",
        required: true,
        flag: "--fee",
        shortFlag: "-f"
    }
];

async function executeUpdateFee(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(updateFeeParams, opts, config, opts.noInteractive);

        printParamSummary("Updating fee:", {
            tokenIn: params.tokenIn,
            tokenOut: params.tokenOut,
            newFee: `${params.fee / 100}% (${params.fee} bps)`
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildUpdateOfferFeeIx({
            tokenInMint: params.tokenIn,
            tokenOutMint: params.tokenOut,
            newFeeBasisPoints: params.fee,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Update Offer Fee Transaction",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeCloseOffer(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(tokenPairParams, opts, config, opts.noInteractive);

        printParamSummary("Closing offer:", {
            tokenIn: params.tokenIn,
            tokenOut: params.tokenOut
        });

        // Confirm dangerous action
        const { confirm } = await import("@inquirer/prompts");
        const confirmed = await confirm({
            message: chalk.yellow("This will permanently close the offer. Continue?"),
            default: false
        });

        if (!confirmed) {
            console.log(chalk.yellow("\nOperation cancelled."));
            return;
        }

        const boss = await helper.getBoss();
        const ix = await helper.buildCloseOfferIx({
            tokenInMint: params.tokenIn,
            tokenOutMint: params.tokenOut,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Close Offer Transaction",
            description: "This will permanently close the offer!",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}
