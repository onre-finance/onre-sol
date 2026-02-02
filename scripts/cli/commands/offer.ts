import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import {
    executeOfferMake,
    executeOfferFetch,
    executeOfferAddVector,
    executeOfferDeleteVector,
    executeOfferUpdateFee,
    executeOfferDeleteAllVectors
} from "../implementations";

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
            await executeOfferMake(opts);
        });

    // offer fetch
    program
        .command("fetch")
        .description("Fetch and display offer details")
        .option("-i, --token-in <mint>", "Token in mint")
        .option("-o, --token-out <mint>", "Token out mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeOfferFetch(opts);
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
            await executeOfferAddVector(opts);
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
            await executeOfferDeleteVector(opts);
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
            await executeOfferUpdateFee(opts);
        });

    // offer delete-all-vectors
    program
        .command("delete-all-vectors")
        .description("Delete all pricing vectors from an offer")
        .option("-i, --token-in <mint>", "Token in mint")
        .option("-o, --token-out <mint>", "Token out mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeOfferDeleteAllVectors(opts);
        });

}

