import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import {
    executeBufferManage,
    executeBufferBurn,
    executeBufferGet,
    executeBufferInitialize,
    executeBufferSetYields,
} from "../implementations";

export function registerBufferCommands(program: Command): void {
    program
        .command("get")
        .description("Fetch BUFFER state")
        .action(async (_, cmd) => {
            const opts = cmd.optsWithGlobals() as GlobalOptions;
            await executeBufferGet(opts);
        });

    program
        .command("initialize")
        .description("Initialize BUFFER state and vault")
        .option("--offer <address>", "Main offer PDA")
        .option("--onyc-mint <address>", "ONyc mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeBufferInitialize(opts);
        });

    program
        .command("set-gross-yield")
        .description("Set BUFFER gross yield")
        .option("--gross-yield <value>", "Gross yield")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeBufferSetYields(opts);
        });

    program
        .command("manage")
        .description("Manage BUFFER spread")
        .option("--offer <address>", "Main offer PDA")
        .option("--onyc-mint <address>", "ONyc mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeBufferManage(opts);
        });

    program
        .command("burn")
        .description("Burn from BUFFER to support NAV increase")
        .option("--token-in <address>", "Offer token-in mint (e.g. USDC)")
        .option("--asset-adjustment-amount <value>", "Asset adjustment amount (raw)")
        .option("--target-nav <value>", "Target NAV (raw)")
        .option("--onyc-mint <address>", "ONyc mint")
        .option("--simulate", "Simulate burn instruction and print computed burn outcome")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeBufferBurn(opts);
        });
}
