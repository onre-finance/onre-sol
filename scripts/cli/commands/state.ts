import { Command } from "commander";
import type { GlobalOptions } from "../prompts";
import {
    executeStateAcceptBoss,
    executeStateAddAdmin,
    executeStateAddApprover,
    executeStateClearAdmins,
    executeStateClose,
    executeStateGet,
    executeStateKillSwitch,
    executeStateMaxSupply,
    executeStateProposeBoss,
    executeStateRemoveAdmin,
    executeStateRemoveApprover,
    executeStateSetOnycMint,
    executeStateSetRedemptionAdmin,
} from "../implementations";

/**
 * Register state subcommands
 */
export function registerStateCommands(program: Command): void {
    // state get
    program
        .command("get")
        .description("Display current program state")
        .action(async (_, cmd) => {
            const opts = cmd.optsWithGlobals() as GlobalOptions;
            await executeStateGet(opts);
        });

    // state propose-boss
    program
        .command("propose-boss")
        .description("Propose a new boss (step 1 of 2)")
        .option("--new-boss <address>", "New boss public key")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateProposeBoss(opts);
        });

    // state accept-boss
    program
        .command("accept-boss")
        .description("Accept boss transfer (step 2 of 2)")
        .option("--new-boss <address>", "New boss public key (must match proposed)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateAcceptBoss(opts);
        });

    // state add-admin
    program
        .command("add-admin")
        .description("Add an admin to the program")
        .option("--admin <address>", "Admin public key to add")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateAddAdmin(opts);
        });

    // state remove-admin
    program
        .command("remove-admin")
        .description("Remove an admin from the program")
        .option("--admin <address>", "Admin public key to remove")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateRemoveAdmin(opts);
        });

    // state add-approver
    program
        .command("add-approver")
        .description("Add an approver to the program")
        .option("--approver <address>", "Approver public key to add")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateAddApprover(opts);
        });

    // state remove-approver
    program
        .command("remove-approver")
        .description("Remove an approver from the program")
        .option("--approver <address>", "Approver public key to remove")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateRemoveApprover(opts);
        });

    // state set-onyc-mint
    program
        .command("set-onyc-mint")
        .description("Set the ONyc mint address")
        .option("--mint <address>", "ONyc mint public key")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateSetOnycMint(opts);
        });

    // state kill-switch
    program
        .command("kill-switch")
        .description("Enable or disable the kill switch")
        .option("--enable", "Enable the kill switch")
        .option("--disable", "Disable the kill switch")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateKillSwitch(opts);
        });

    // state max-supply
    program
        .command("max-supply")
        .description("Configure maximum ONyc supply")
        .option("--amount <value>", "Maximum supply amount (raw)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateMaxSupply(opts);
        });

    // state set-redemption-admin
    program
        .command("set-redemption-admin")
        .description("Set the redemption admin who can fulfill redemption requests")
        .option("--admin <address>", "Redemption admin public key")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateSetRedemptionAdmin(opts);
        });

    // state clear-admins
    program
        .command("clear-admins")
        .description("Remove all admins from the program")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateClearAdmins(opts);
        });

    // state close
    program
        .command("close")
        .description("Close the program state account (DANGEROUS)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeStateClose(opts);
        });
}
