import { Command } from "commander";
import chalk from "chalk";
import { PublicKey } from "@solana/web3.js";
import { config, NetworkConfig, ScriptHelper } from "../../utils/script-helper";
import type { GlobalOptions } from "../prompts";
import { ParamDefinition, promptForParams } from "../prompts";
import { handleTransaction } from "../transaction/handler";
import { printNetworkBanner, printParamSummary, printState } from "../utils/display";

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
            await executeGetState(opts);
        });

    // state propose-boss
    program
        .command("propose-boss")
        .description("Propose a new boss (step 1 of 2)")
        .option("--new-boss <address>", "New boss public key")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeProposeBoss(opts);
        });

    // state accept-boss
    program
        .command("accept-boss")
        .description("Accept boss transfer (step 2 of 2)")
        .option("--new-boss <address>", "New boss public key (must match proposed)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeAcceptBoss(opts);
        });

    // state add-admin
    program
        .command("add-admin")
        .description("Add an admin to the program")
        .option("--admin <address>", "Admin public key to add")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeAddAdmin(opts);
        });

    // state remove-admin
    program
        .command("remove-admin")
        .description("Remove an admin from the program")
        .option("--admin <address>", "Admin public key to remove")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRemoveAdmin(opts);
        });

    // state add-approver
    program
        .command("add-approver")
        .description("Add an approver to the program")
        .option("--approver <address>", "Approver public key to add")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeAddApprover(opts);
        });

    // state remove-approver
    program
        .command("remove-approver")
        .description("Remove an approver from the program")
        .option("--approver <address>", "Approver public key to remove")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeRemoveApprover(opts);
        });

    // state set-onyc-mint
    program
        .command("set-onyc-mint")
        .description("Set the ONyc mint address")
        .option("--mint <address>", "ONyc mint public key")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeSetOnycMint(opts);
        });

    // state kill-switch
    program
        .command("kill-switch")
        .description("Enable or disable the kill switch")
        .option("--enable", "Enable the kill switch")
        .option("--disable", "Disable the kill switch")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeKillSwitch(opts);
        });

    // state max-supply
    program
        .command("max-supply")
        .description("Configure maximum ONyc supply")
        .option("--amount <value>", "Maximum supply amount (raw)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeConfigureMaxSupply(opts);
        });

    // state set-redemption-admin
    program
        .command("set-redemption-admin")
        .description("Set the redemption admin who can fulfill redemption requests")
        .option("--admin <address>", "Redemption admin public key")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeSetRedemptionAdmin(opts);
        });

    // state clear-admins
    program
        .command("clear-admins")
        .description("Remove all admins from the program")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeClearAdmins(opts);
        });

    // state close
    program
        .command("close")
        .description("Close the program state account (DANGEROUS - reclaims rent)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeCloseState(opts);
        });
}

// === Command Implementations ===

async function executeGetState(opts: GlobalOptions): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const state = await helper.getState();

        printState(state, opts.json);
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

const proposeBossParams: ParamDefinition[] = [
    {
        name: "newBoss",
        type: "publicKey",
        description: "New boss public key",
        required: true,
        flag: "--new-boss"
    }
];

async function executeProposeBoss(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(proposeBossParams, opts, config, opts.noInteractive);

        printParamSummary("Proposing new boss:", { newBoss: params.newBoss });

        const boss = await helper.getBoss();
        const ix = await helper.buildProposeBossIx({ newBoss: params.newBoss, boss });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Propose Boss Transaction",
            description: "Proposes a new boss. The new boss must call accept-boss to complete the transfer.",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

const acceptBossParams: ParamDefinition[] = [
    {
        name: "newBoss",
        type: "publicKey",
        description: "New boss public key (must match proposed boss)",
        required: true,
        flag: "--new-boss"
    }
];

async function executeAcceptBoss(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(acceptBossParams, opts, config, opts.noInteractive);

        printParamSummary("Accepting boss transfer:", { newBoss: params.newBoss });

        const ix = await helper.buildAcceptBossIx({ newBoss: params.newBoss });
        const tx = await helper.prepareTransaction({ ix, payer: params.newBoss });

        await handleTransaction(tx, helper, {
            title: "Accept Boss Transaction",
            description: "Completes the boss transfer. Must be signed by the proposed boss.",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

const adminParams: ParamDefinition[] = [
    {
        name: "admin",
        type: "publicKey",
        description: "Admin public key",
        required: true,
        flag: "--admin"
    }
];

async function executeAddAdmin(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(adminParams, opts, config, opts.noInteractive);

        printParamSummary("Adding admin:", { admin: params.admin });

        const boss = await helper.getBoss();
        const ix = await helper.buildAddAdminIx({ admin: params.admin, boss });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Add Admin Transaction",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeRemoveAdmin(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(adminParams, opts, config, opts.noInteractive);

        printParamSummary("Removing admin:", { admin: params.admin });

        const boss = await helper.getBoss();
        const ix = await helper.buildRemoveAdminIx({ admin: params.admin, boss });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Remove Admin Transaction",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

const approverParams: ParamDefinition[] = [
    {
        name: "approver",
        type: "publicKey",
        description: "Approver public key",
        required: true,
        flag: "--approver"
    }
];

async function executeAddApprover(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(approverParams, opts, config, opts.noInteractive);

        printParamSummary("Adding approver:", { approver: params.approver });

        const boss = await helper.getBoss();
        const ix = await helper.buildAddApproverIx({ approver: params.approver, boss });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Add Approver Transaction",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeRemoveApprover(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(approverParams, opts, config, opts.noInteractive);

        printParamSummary("Removing approver:", { approver: params.approver });

        const boss = await helper.getBoss();
        const ix = await helper.buildRemoveApproverIx({ approver: params.approver, boss });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Remove Approver Transaction",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

const setOnycMintParams: ParamDefinition[] = [
    {
        name: "mint",
        type: "mint",
        description: "ONyc mint address",
        required: true,
        flag: "--mint",
        default: (cfg: NetworkConfig) => cfg.mints.onyc
    }
];

async function executeSetOnycMint(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(setOnycMintParams, opts, config, opts.noInteractive);

        printParamSummary("Setting ONyc mint:", { onycMint: params.mint });

        const boss = await helper.getBoss();
        const ix = await helper.buildSetOnycMintIx({ onycMint: params.mint, boss });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Set ONyc Mint Transaction",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeKillSwitch(opts: GlobalOptions & { enable?: boolean; disable?: boolean }): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        // Determine enable/disable
        let enable: boolean;
        if (opts.enable) {
            enable = true;
        } else if (opts.disable) {
            enable = false;
        } else {
            // Prompt for choice
            const { select } = await import("@inquirer/prompts");
            const choice = await select({
                message: "Kill switch action:",
                choices: [
                    { name: "Enable (stop all operations)", value: "enable" },
                    { name: "Disable (resume operations)", value: "disable" }
                ]
            });
            enable = choice === "enable";
        }

        const helper = await ScriptHelper.create();

        printParamSummary("Kill switch operation:", {
            action: enable ? "ENABLE" : "DISABLE"
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildSetKillSwitchIx({ enable, boss });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: enable ? "Enable Kill Switch Transaction" : "Disable Kill Switch Transaction",
            description: enable
                ? "This will STOP all program operations!"
                : "This will resume program operations.",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

const maxSupplyParams: ParamDefinition[] = [
    {
        name: "amount",
        type: "amount",
        description: "Maximum supply amount (raw, with 9 decimals)",
        required: true,
        flag: "--amount"
    }
];

async function executeConfigureMaxSupply(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(maxSupplyParams, opts, config, opts.noInteractive);

        printParamSummary("Configuring max supply:", {
            maxSupply: params.amount,
            displayAmount: `${(params.amount / 1_000_000_000).toLocaleString()} tokens`
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildConfigureMaxSupplyIx({ maxSupply: params.amount, boss });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Configure Max Supply Transaction",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

const redemptionAdminParams: ParamDefinition[] = [
    {
        name: "redemptionAdmin",
        type: "publicKey",
        description: "Redemption admin public key",
        required: true,
        flag: "--admin"
    }
];

async function executeSetRedemptionAdmin(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(redemptionAdminParams, opts, config, opts.noInteractive);

        printParamSummary("Setting redemption admin:", { redemptionAdmin: params.redemptionAdmin });

        const boss = await helper.getBoss();
        const ix = await helper.buildSetRedemptionAdminIx({
            redemptionAdmin: params.redemptionAdmin,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Set Redemption Admin Transaction",
            description: "Sets the admin who can fulfill redemption requests.",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeClearAdmins(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();

        // Get current state to show admin count
        const state = await helper.getState();
        const adminCount = state.admins.filter((admin: any) => !admin.equals(PublicKey.default)).length;

        if (adminCount === 0 && !opts.json) {
            console.log(chalk.yellow("No admins to clear."));
            return;
        }

        // Confirmation prompt for dangerous operation
        if (!opts.noInteractive && !opts.dryRun) {
            const { confirm } = await import("@inquirer/prompts");
            const confirmed = await confirm({
                message: `Remove all ${adminCount} admin(s)? This cannot be undone.`,
                default: false
            });
            if (!confirmed) {
                console.log(chalk.yellow("Operation cancelled"));
                return;
            }
        }

        printParamSummary("Clearing all admins:", {
            adminCount: `${adminCount} admin(s) will be removed`
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildClearAdminsIx({ boss });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Clear Admins Transaction",
            description: `Removes all ${adminCount} admin(s) from the program.`,
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeCloseState(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();

        // STRONG warning for dangerous operation
        if (!opts.noInteractive && !opts.dryRun) {
            console.log(chalk.red.bold("\n⚠️  DANGER: This will permanently close the program state account!"));
            console.log(chalk.red("This operation is IRREVERSIBLE and will reclaim rent."));
            console.log(chalk.yellow("The program will no longer be usable after this operation.\n"));

            const { input } = await import("@inquirer/prompts");
            const confirmation = await input({
                message: "Type 'CLOSE' (in uppercase) to confirm:"
            });

            if (confirmation !== "CLOSE") {
                console.log(chalk.yellow("Operation cancelled - confirmation did not match."));
                return;
            }
        }

        printParamSummary("Closing program state:", {
            warning: "⚠️  This will permanently close the program state!"
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildCloseStateIx({ boss });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Close State Transaction",
            description: "⚠️  PERMANENTLY closes the program state account and reclaims rent.",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}
