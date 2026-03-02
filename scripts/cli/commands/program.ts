import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import { executeProgramExtend } from "../implementations";

/**
 * Register program subcommands
 */
export function registerProgramCommands(program: Command): void {
    // program extend
    program
        .command("extend")
        .description("Extend program data account size")
        .option("-b, --bytes <number>", "Additional bytes to allocate")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeProgramExtend(opts);
        });
}
