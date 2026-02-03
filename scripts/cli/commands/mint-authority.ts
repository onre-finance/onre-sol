import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import {
    executeMintTo,
    executeMintAuthorityToProgram,
    executeMintAuthorityToBoss
} from "../implementations";

/**
 * Register mint-authority subcommands
 */
export function registerMintAuthorityCommands(program: Command): void {
    // mint-authority mint-to
    program
        .command("mint-to")
        .description("Mint tokens directly to the boss's account")
        .option("-m, --mint <address>", "Mint address")
        .option("-a, --amount <number>", "Amount to mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeMintTo(opts);
        });

    // mint-authority to-program
    program
        .command("to-program")
        .description("Transfer mint authority to the program PDA")
        .option("-m, --mint <address>", "Mint address")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeMintAuthorityToProgram(opts);
        });

    // mint-authority to-boss
    program
        .command("to-boss")
        .description("Transfer mint authority back to the boss")
        .option("-m, --mint <address>", "Mint address")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeMintAuthorityToBoss(opts);
        });
}
