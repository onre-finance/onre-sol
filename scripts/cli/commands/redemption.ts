import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import {
    executeRedemptionCancel,
    executeRedemptionCreateRequest,
    executeRedemptionFetchOffer,
    executeRedemptionFetchRequest,
    executeRedemptionFulfill,
    executeRedemptionListRequests,
    executeRedemptionMakeOffer,
    executeRedemptionUpdateFee,
} from "../implementations";

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
            await executeRedemptionMakeOffer(opts);
        });

    // redemption fetch-offer
    program
        .command("fetch-offer")
        .description("Fetch and display redemption offer details")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRedemptionFetchOffer(opts);
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
            await executeRedemptionUpdateFee(opts);
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
            await executeRedemptionCreateRequest(opts);
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
            await executeRedemptionFetchRequest(opts);
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
            await executeRedemptionFulfill(opts);
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
            await executeRedemptionCancel(opts);
        });

    // redemption list-requests
    program
        .command("list-requests")
        .description("List all redemption requests for an offer")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRedemptionListRequests(opts);
        });
}
