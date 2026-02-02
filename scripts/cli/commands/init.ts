import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import {
    executeInitProgram,
    executeInitPermissionless
} from "../implementations";

/**
 * Register init subcommands
 */
export function registerInitCommands(program: Command): void {
    // init program
    program
        .command("program")
        .description("Initialize the program state")
        .option("--onyc-mint <address>", "ONyc mint address")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeInitProgram(opts);
        });

    // init permissionless
    program
        .command("permissionless")
        .description("Initialize the permissionless vault authority")
        .option("--name <name>", "Authority name (e.g., 'permissionless-1')")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeInitPermissionless(opts);
        });
}
