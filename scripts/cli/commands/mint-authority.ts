import { Command } from "commander";
import chalk from "chalk";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { config, NetworkConfig, ScriptHelper } from "../../utils/script-helper";
import type { GlobalOptions } from "../prompts";
import { ParamDefinition, promptForParams } from "../prompts";
import { handleTransaction } from "../transaction/handler";
import { printNetworkBanner, printParamSummary } from "../utils/display";

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
            await executeTransferToProgram(opts);
        });

    // mint-authority to-boss
    program
        .command("to-boss")
        .description("Transfer mint authority back to the boss")
        .option("-m, --mint <address>", "Mint address")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeTransferToBoss(opts);
        });
}

// === Command Implementations ===

const mintParams: ParamDefinition[] = [
    {
        name: "mint",
        type: "mint",
        description: "Mint address",
        required: true,
        flag: "--mint",
        shortFlag: "-m",
        default: (cfg: NetworkConfig) => cfg.mints.onyc
    },
    {
        name: "amount",
        type: "amount",
        description: "Amount to mint",
        required: true,
        flag: "--amount",
        shortFlag: "-a"
    }
];

async function executeMintTo(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(mintParams, opts, config, opts.noInteractive);

        printParamSummary("Minting tokens to boss:", {
            mint: params.mint,
            amount: params.amount
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildMintToIx({
            amount: params.amount
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Mint To Transaction",
            description: `Mints ${params.amount} tokens of ${params.mint.toBase58().slice(0, 12)}... to boss`,
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

const transferMintAuthorityParams: ParamDefinition[] = [
    {
        name: "mint",
        type: "mint",
        description: "Mint address",
        required: true,
        flag: "--mint",
        shortFlag: "-m",
        default: (cfg: NetworkConfig) => cfg.mints.onyc
    }
];

async function executeTransferToProgram(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(transferMintAuthorityParams, opts, config, opts.noInteractive);

        printParamSummary("Transferring mint authority to program:", {
            mint: params.mint,
            targetAuthority: helper.pdas.mintAuthorityPda
        });

        // Confirm dangerous action
        const { confirm } = await import("@inquirer/prompts");
        const confirmed = await confirm({
            message: chalk.yellow("This will generate a transaction to transfer mint authority to the program. Continue?"),
            default: false
        });

        if (!confirmed) {
            console.log(chalk.yellow("\nOperation cancelled."));
            return;
        }

        const boss = await helper.getBoss();
        const ix = await helper.buildTransferMintAuthorityToProgramIx({
            mint: params.mint,
            tokenProgram: TOKEN_PROGRAM_ID,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Transfer Mint Authority to Program",
            description: `Transfers mint authority of ${params.mint.toBase58().slice(0, 12)}... to program PDA`,
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeTransferToBoss(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(mintParams, opts, config, opts.noInteractive);

        const boss = await helper.getBoss();

        printParamSummary("Transferring mint authority to boss:", {
            mint: params.mint,
            targetAuthority: boss
        });

        const ix = await helper.buildTransferMintAuthorityToBossIx({
            mint: params.mint,
            tokenProgram: TOKEN_PROGRAM_ID,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Transfer Mint Authority to Boss",
            description: `Transfers mint authority of ${params.mint.toBase58().slice(0, 12)}... back to boss`,
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}
