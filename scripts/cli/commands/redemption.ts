import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import {
    executeRedemptionCancel,
    executeRedemptionCreateRequest,
    executeRedemptionFetchOffer,
    executeRedemptionFetchRequest,
    executeRedemptionFetchVaults,
    executeRedemptionFulfill,
    executeRedemptionListOffers,
    executeRedemptionListRequests,
    executeRedemptionMakeOffer,
    executeRedemptionSetDisabled,
    executeRedemptionUpdateFee,
    executeRedemptionUpdatePropAmmSellFee,
    executeRedemptionUpdateVaultTarget,
} from "../implementations";

/**
 * Register redemption subcommands
 */
export function registerRedemptionCommands(program: Command): void {
    // redemption list-offers
    program
        .command("list-offers")
        .description("List all redemption offers on-chain")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRedemptionListOffers(opts);
        });

    // redemption make-offer
    program
        .command("make-offer")
        .description("Create a redemption offer (ONyc -> USDC)")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("-f, --fee <bps>", "Fee in basis points")
        .option("--prop-amm-sell-fee <bps>", "Prop AMM sell redemption fee in basis points")
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
        .description("Update worker fulfillment fee")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("-f, --fee <bps>", "New fee in basis points")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRedemptionUpdateFee(opts);
        });

    // redemption update-prop-amm-sell-fee
    program
        .command("update-prop-amm-sell-fee")
        .description("Update redemption fee used by Prop AMM sells")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("-f, --fee <bps>", "New fee in basis points")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRedemptionUpdatePropAmmSellFee(opts);
        });

    // redemption update-vault-target
    program
        .command("update-vault-target")
        .description("Update redemption offer vault target")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("--target-bps <bps>", "New vault target in basis points")
        .action(async (options, cmd) => {
            const opts = { ...options, vaultTargetBps: options.targetBps, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRedemptionUpdateVaultTarget(opts);
        });

    // redemption set-disabled
    program
        .command("set-disabled")
        .description("Enable or disable a redemption offer")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("--disabled <boolean>", "Disabled state (true/false)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRedemptionSetDisabled(opts);
        });

    // redemption create-request
    program
        .command("create-request")
        .description("Create a redemption request (locks tokens)")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("-a, --amount <tokens>", "Amount of tokens to redeem")
        .option("--request-id <string>", "Frontend-generated random request ID")
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
        .option("--request-id <string>", "Frontend-generated request ID")
        .option("--redeemer <address>", "Redeemer public key")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRedemptionFetchRequest(opts);
        });

    // redemption fulfill
    program
        .command("fulfill")
        .description("Fulfill a pending redemption request (worker only)")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("--request-id <string>", "Frontend-generated request ID")
        .option("--redeemer <address>", "Redeemer public key")
        .option("-a, --amount <amount>", "Amount to fulfill (omit for full remaining)")
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
        .option("--request-id <string>", "Frontend-generated request ID")
        .option("--redeemer <address>", "Redeemer public key")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRedemptionCancel(opts);
        });

    // redemption fetch-vaults
    program
        .command("fetch-vaults")
        .description("Fetch and display all redemption vault balances")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRedemptionFetchVaults(opts);
        });

    // redemption list-requests
    program
        .command("list-requests")
        .description("List all redemption requests for an offer")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("-r, --redeemer <address>", "Filter by redeemer address (optional)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRedemptionListRequests(opts);
        });
}
