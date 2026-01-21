import { Command } from "commander";
import chalk from "chalk";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { config, NetworkConfig, ScriptHelper } from "../../utils/script-helper";
import type { GlobalOptions } from "../prompts";
import { ParamDefinition, promptForParams } from "../prompts";
import { handleTransaction } from "../transaction/handler";
import { printNetworkBanner, printParamSummary, printRedemptionOffer, printRedemptionRequest, printRedemptionRequestsList } from "../utils/display";

// Common token pair params
const tokenPairParams: ParamDefinition[] = [
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

const redemptionOfferParams: ParamDefinition[] = [
    ...tokenPairParams,
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

const createRequestParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "amount",
        type: "amount",
        description: "Amount of tokens to redeem",
        required: true,
        flag: "--amount",
        shortFlag: "-a"
    }
];

const requestParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "requestId",
        type: "string",
        description: "Redemption request ID",
        required: true,
        flag: "--request-id",
        transform: (value: any) => parseInt(value, 10)
    }
];

const updateFeeParams: ParamDefinition[] = [
    ...tokenPairParams,
    {
        name: "fee",
        type: "basisPoints",
        description: "New redemption fee in basis points",
        required: true,
        flag: "--fee",
        shortFlag: "-f"
    }
];

/**
 * Register redemption subcommands
 */
export function registerRedemptionCommands(program: Command): void {
    // redemption make-offer
    program
        .command("make-offer")
        .description("Create a redemption offer (ONyc -> USDC)")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("-f, --fee <bps>", "Fee in basis points")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeMakeRedemptionOffer(opts);
        });

    // redemption fetch-offer
    program
        .command("fetch-offer")
        .description("Fetch and display redemption offer details")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeFetchRedemptionOffer(opts);
        });

    // redemption update-fee
    program
        .command("update-fee")
        .description("Update redemption offer fee")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("-f, --fee <bps>", "New fee in basis points")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeUpdateRedemptionFee(opts);
        });

    // redemption create-request
    program
        .command("create-request")
        .description("Create a redemption request (locks tokens)")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("-a, --amount <tokens>", "Amount of tokens to redeem")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeCreateRedemptionRequest(opts);
        });

    // redemption fetch-request
    program
        .command("fetch-request")
        .description("Fetch and display redemption request details")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("--request-id <number>", "Request ID")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeFetchRedemptionRequest(opts);
        });

    // redemption fulfill
    program
        .command("fulfill")
        .description("Fulfill a pending redemption request (redemption admin only)")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("--request-id <number>", "Request ID")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeFulfillRedemptionRequest(opts);
        });

    // redemption cancel
    program
        .command("cancel")
        .description("Cancel a redemption request (returns locked tokens)")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("--request-id <number>", "Request ID")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeCancelRedemptionRequest(opts);
        });

    // redemption list-requests
    program
        .command("list-requests")
        .description("List all redemption requests for an offer")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeListRedemptionRequests(opts);
        });
}

// === Command Implementations ===

async function executeMakeRedemptionOffer(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(redemptionOfferParams, opts, config, opts.noInteractive);

        printParamSummary("Creating redemption offer:", {
            tokenIn: params.tokenIn,
            tokenOut: params.tokenOut,
            fee: `${params.fee / 100}% (${params.fee} bps)`
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildMakeRedemptionOfferIx({
            tokenInMint: params.tokenIn,
            tokenInProgram: TOKEN_PROGRAM_ID,
            tokenOutMint: params.tokenOut,
            tokenOutProgram: TOKEN_PROGRAM_ID,
            feeBasisPoints: params.fee,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Make Redemption Offer Transaction",
            description: "Creates a redemption offer allowing ONyc holders to redeem for USDC",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeFetchRedemptionOffer(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(tokenPairParams, opts, config, opts.noInteractive);

        const offer = await helper.fetchRedemptionOffer(params.tokenIn, params.tokenOut);

        printRedemptionOffer(offer, params.tokenIn.toString(), params.tokenOut.toString(), opts.json);
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeUpdateRedemptionFee(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(updateFeeParams, opts, config, opts.noInteractive);

        const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);

        printParamSummary("Updating redemption fee:", {
            tokenIn: params.tokenIn,
            tokenOut: params.tokenOut,
            newFee: `${params.fee / 100}% (${params.fee} bps)`
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildUpdateRedemptionOfferFeeIx({
            redemptionOfferPda,
            newFeeBasisPoints: params.fee,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Update Redemption Fee Transaction",
            description: "Updates the fee for redemption requests",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeCreateRedemptionRequest(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(createRequestParams, opts, config, opts.noInteractive);

        // Get the wallet public key from the provider
        const redeemer = helper.program.provider.publicKey!;
        const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);

        printParamSummary("Creating redemption request:", {
            tokenIn: params.tokenIn,
            tokenOut: params.tokenOut,
            amount: params.amount,
            redeemer: redeemer.toString()
        });

        const ix = await helper.buildCreateRedemptionRequestIx({
            redemptionOfferPda,
            tokenInMint: params.tokenIn,
            amount: params.amount,
            redeemer
        });
        const tx = await helper.prepareTransaction({ ix, payer: redeemer });

        await handleTransaction(tx, helper, {
            title: "Create Redemption Request Transaction",
            description: "Creates a redemption request and locks tokens in vault",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeFetchRedemptionRequest(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(requestParams, opts, config, opts.noInteractive);

        const offerPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);
        const request = await helper.fetchRedemptionRequest(offerPda, params.requestId);

        printRedemptionRequest(request, params.requestId, opts.json);
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeFulfillRedemptionRequest(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(requestParams, opts, config, opts.noInteractive);

        const state = await helper.getState();
        const redemptionAdmin = state.redemptionAdmin;

        const offerPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);
        const requestPda = helper.getRedemptionRequestPda(offerPda, params.requestId);

        printParamSummary("Fulfilling redemption request:", {
            requestId: params.requestId,
            redemptionAdmin: redemptionAdmin.toString()
        });

        const ix = await helper.buildFulfillRedemptionRequestIx({
            redemptionOfferPda: offerPda,
            redemptionRequestPda: requestPda,
            redemptionAdmin
        });
        const tx = await helper.prepareTransaction({ ix, payer: redemptionAdmin });

        await handleTransaction(tx, helper, {
            title: "Fulfill Redemption Request Transaction",
            description: "Executes redemption at current NAV price",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeCancelRedemptionRequest(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(requestParams, opts, config, opts.noInteractive);

        // Get the signer (can be redeemer, redemption_admin, or boss)
        const signer = helper.program.provider.publicKey!;

        const offerPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);
        const requestPda = helper.getRedemptionRequestPda(offerPda, params.requestId);

        printParamSummary("Cancelling redemption request:", {
            requestId: params.requestId,
            signer: signer.toString()
        });

        const ix = await helper.buildCancelRedemptionRequestIx({
            redemptionOfferPda: offerPda,
            redemptionRequestPda: requestPda,
            signer
        });
        const tx = await helper.prepareTransaction({ ix, payer: signer });

        await handleTransaction(tx, helper, {
            title: "Cancel Redemption Request Transaction",
            description: "Cancels redemption request and returns locked tokens",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeListRedemptionRequests(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(tokenPairParams, opts, config, opts.noInteractive);

        const offerPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);

        // Fetch all redemption requests for this offer using memcmp filter
        // This is much more efficient than iterating through all request IDs
        const accounts = await helper.program.account.redemptionRequest.all([
            {
                memcmp: {
                    offset: 8, // Skip 8-byte discriminator
                    bytes: offerPda.toBase58()
                }
            }
        ]);

        // Map to the format expected by the display function
        const requests = accounts.map(acc => ({
            id: acc.account.requestId.toNumber(),
            request: acc.account
        }));

        // Sort by request ID for consistent display
        requests.sort((a, b) => a.id - b.id);

        printRedemptionRequestsList(requests, opts.json);
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}
