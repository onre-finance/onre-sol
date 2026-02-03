import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import {
    executeVaultDeposit,
    executeVaultWithdraw,
    executeVaultRedemptionDeposit,
    executeVaultRedemptionWithdraw
} from "../implementations";

/**
 * Register vault subcommands
 */
export function registerVaultCommands(program: Command): void {
    // vault deposit
    program
        .command("deposit")
        .description("Deposit tokens to the offer vault")
        .option("-t, --token <mint>", "Token mint")
        .option("-a, --amount <value>", "Amount to deposit (raw)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeVaultDeposit(opts);
        });

    // vault withdraw
    program
        .command("withdraw")
        .description("Withdraw tokens from the offer vault")
        .option("-t, --token <mint>", "Token mint")
        .option("-a, --amount <value>", "Amount to withdraw (raw)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeVaultWithdraw(opts);
        });

    // vault redemption-deposit
    program
        .command("redemption-deposit")
        .description("Deposit tokens to the redemption vault")
        .option("-t, --token <mint>", "Token mint")
        .option("-a, --amount <value>", "Amount to deposit (raw)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeVaultRedemptionDeposit(opts);
        });

    // vault redemption-withdraw
    program
        .command("redemption-withdraw")
        .description("Withdraw tokens from the redemption vault")
        .option("-t, --token <mint>", "Token mint")
        .option("-a, --amount <value>", "Amount to withdraw (raw)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeVaultRedemptionWithdraw(opts);
        });
}
