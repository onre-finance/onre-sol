import { Command } from "commander";
import chalk from "chalk";
import { PublicKey } from "@solana/web3.js";
import { config, NetworkConfig, ScriptHelper } from "../../utils/script-helper";
import type { GlobalOptions } from "../prompts";
import { ParamDefinition, promptForParams } from "../prompts";
import { handleTransaction } from "../transaction/handler";
import { printNetworkBanner, printParamSummary } from "../utils/display";

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
            await executeInitializeProgram(opts);
        });

    // init permissionless
    program
        .command("permissionless")
        .description("Initialize the permissionless vault authority")
        .option("--name <name>", "Authority name (e.g., 'permissionless-1')")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeInitializePermissionless(opts);
        });
}

// === Command Implementations ===

const initProgramParams: ParamDefinition[] = [
    {
        name: "onycMint",
        type: "mint",
        description: "ONyc mint address",
        required: true,
        flag: "--onyc-mint",
        default: (cfg: NetworkConfig) => cfg.mints.onyc
    }
];

async function executeInitializeProgram(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(initProgramParams, opts, config, opts.noInteractive);

        // Get program data PDA
        const [programDataPda] = PublicKey.findProgramAddressSync(
            [config.programId.toBuffer()],
            new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111")
        );

        printParamSummary("Initializing program:", {
            programId: config.programId,
            onycMint: params.onycMint,
            programData: programDataPda,
            boss: config.boss
        });

        // Confirm action
        const { confirm } = await import("@inquirer/prompts");
        const confirmed = await confirm({
            message: chalk.yellow("This will initialize the program state. Continue?"),
            default: false
        });

        if (!confirmed) {
            console.log(chalk.yellow("\nOperation cancelled."));
            return;
        }

        const ix = await helper.buildInitializeIx({
            boss: config.boss,
            programData: programDataPda,
            onycMint: params.onycMint
        });
        const tx = await helper.prepareTransaction({ ix, payer: config.boss });

        await handleTransaction(tx, helper, {
            title: "Initialize Program Transaction",
            description: "Initializes the program state account",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error);
        process.exit(1);
    }
}

const initPermissionlessParams: ParamDefinition[] = [
    {
        name: "name",
        type: "string",
        description: "Authority name (e.g., 'permissionless-1')",
        required: true,
        flag: "--name",
        default: "permissionless-1"
    }
];

async function executeInitializePermissionless(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(initPermissionlessParams, opts, config, opts.noInteractive);

        printParamSummary("Initializing permissionless authority:", {
            name: params.name,
            boss: config.boss
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildInitializePermissionlessAuthorityIx({
            name: params.name,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Initialize Permissionless Authority Transaction",
            description: `Creates permissionless vault authority with name: ${params.name}`,
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}
