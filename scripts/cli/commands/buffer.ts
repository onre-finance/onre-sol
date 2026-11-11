import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import {
    executeBufferBurn,
    executeBufferGet,
    executeBufferInitialize,
    executeBufferReserveDeposit,
    executeBufferReserveWithdraw,
    executeBufferSetFees,
    executeBufferSetYields,
    executeBufferSettle,
} from "../implementations";

export function registerBufferCommands(program: Command): void {
    program.addHelpText("after", "\nThese commands target the legacy ONyc BUFFER. After an explicit ONyc BUFFER cutover, use the modular settlement/settings instructions and per-mint vault operations from the generated IDL.");
    program
        .command("get")
        .description("Fetch legacy ONyc BUFFER state")
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
        .command("set-fees")
        .description("Set BUFFER management and performance fees and high-water-mark behavior")
        .option("--management-fee-bps <bps>", "Management fee in basis points")
        .option("--performance-fee-bps <bps>", "Performance fee in basis points")
        .option("--performance-fee-high-watermark-enabled <boolean>", "Enable the performance-fee high-water mark (true or false)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeBufferSetFees(opts);
        });

    program
        .command("settle")
        .description("Settle BUFFER accrual (worker only)")
        .action(async (_, cmd) => {
            const opts = cmd.optsWithGlobals() as GlobalOptions;
            await executeBufferSettle(opts);
        });

    program
        .command("reserve-deposit")
        .description("Deposit ONyc into the BUFFER reserve vault")
        .option("--onyc-mint <address>", "ONyc mint")
        .option("-a, --amount <value>", "Amount to deposit (raw)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeBufferReserveDeposit(opts);
        });

    program
        .command("reserve-withdraw")
        .description("Withdraw ONyc from the BUFFER reserve vault")
        .option("--onyc-mint <address>", "ONyc mint")
        .option("-a, --amount <value>", "Amount to withdraw (raw)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeBufferReserveWithdraw(opts);
        });

    program
        .command("burn")
        .description("Burn from BUFFER to support NAV increase")
        .option("--token-in <address>", "Offer token-in mint (e.g. USDC or USDT)")
        .option("--asset-adjustment-amount <value>", "Asset adjustment amount (raw)")
        .option("--target-nav <value>", "Target NAV (raw)")
        .option("--onyc-mint <address>", "ONyc mint")
        .option("--simulate", "Simulate burn instruction and print computed burn outcome")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeBufferBurn(opts);
        });
}
