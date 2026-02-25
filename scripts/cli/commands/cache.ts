import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import {
    executeCacheAccrue,
    executeCacheBurn,
    executeCacheGet,
    executeCacheInitialize,
    executeCacheSetAdmin,
    executeCacheSetYields,
    executeCacheUpdateLowestSupply,
} from "../implementations";

export function registerCacheCommands(program: Command): void {
    program
        .command("get")
        .description("Fetch CACHE state")
        .action(async (_, cmd) => {
            const opts = cmd.optsWithGlobals() as GlobalOptions;
            await executeCacheGet(opts);
        });

    program
        .command("initialize")
        .description("Initialize CACHE state and vault")
        .option("--cache-admin <address>", "CACHE admin public key")
        .option("--onyc-mint <address>", "ONyc mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeCacheInitialize(opts);
        });

    program
        .command("set-admin")
        .description("Set CACHE admin")
        .option("--cache-admin <address>", "CACHE admin public key")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeCacheSetAdmin(opts);
        });

    program
        .command("set-yields")
        .description("Set CACHE gross and current yields (scale=1e6)")
        .option("--gross-yield <value>", "Gross yield")
        .option("--current-yield <value>", "Current yield")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeCacheSetYields(opts);
        });

    program
        .command("update-lowest-supply")
        .description("Update CACHE lowest observed ONyc supply")
        .option("--onyc-mint <address>", "ONyc mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeCacheUpdateLowestSupply(opts);
        });

    program
        .command("accrue")
        .description("Accrue CACHE spread (signer must be cache_admin)")
        .option("--onyc-mint <address>", "ONyc mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeCacheAccrue(opts);
        });

    program
        .command("burn")
        .description("Burn from CACHE to support NAV increase")
        .option("--token-in <address>", "Offer token-in mint (e.g. USDC)")
        .option("--asset-adjustment-amount <value>", "Asset adjustment amount (raw)")
        .option("--target-nav <value>", "Target NAV (raw)")
        .option("--onyc-mint <address>", "ONyc mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeCacheBurn(opts);
        });
}
