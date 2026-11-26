import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import { executeMarketFetch, executeMarketRefresh } from "../implementations";

/**
 * Register market subcommands
 */
export function registerMarketCommands(program: Command): void {
    program
        .command("fetch")
        .description("Read the cached MarketStats PDA for the main offer (no transaction)")
        .action(async (_, cmd) => {
            await executeMarketFetch(cmd.optsWithGlobals() as GlobalOptions);
        });

    // market refresh
    program
        .command("refresh")
        .description("Refresh cached market stats")
        .option("-i, --token-in <mint>", "Token in mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeMarketRefresh(opts);
        });
}
