import { Command } from "commander";
import chalk from "chalk";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { config, ScriptHelper } from "../../utils/script-helper";
import type { GlobalOptions } from "../prompts";
import { ParamDefinition, promptForParams } from "../prompts";
import { handleTransaction } from "../transaction/handler";
import { printNetworkBanner, printParamSummary } from "../utils/display";

const vaultParams: ParamDefinition[] = [
    {
        name: "tokenMint",
        type: "mint",
        description: "Token mint to deposit/withdraw",
        required: true,
        flag: "--token",
        shortFlag: "-t"
    },
    {
        name: "amount",
        type: "amount",
        description: "Amount (raw, with decimals)",
        required: true,
        flag: "--amount",
        shortFlag: "-a"
    }
];

/**
 * Register vault subcommands
 */
export function registerVaultCommands(program: Command): void {
    // vault deposit
    program
        .command("deposit")
        .description("Deposit tokens to the offer vault")
        .option("-t, --token <mint>", "Token mint")
        .option("-a, --amount <value>", "Amount to deposit (raw)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeDeposit(opts);
        });

    // vault withdraw
    program
        .command("withdraw")
        .description("Withdraw tokens from the offer vault")
        .option("-t, --token <mint>", "Token mint")
        .option("-a, --amount <value>", "Amount to withdraw (raw)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeWithdraw(opts);
        });
}

// === Command Implementations ===

async function executeDeposit(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(vaultParams, opts, config, opts.noInteractive);

        // Determine decimals based on known mints
        let decimals = 9;
        if (params.tokenMint.equals(config.mints.usdc)) {
            decimals = 6;
        }

        printParamSummary("Depositing to vault:", {
            tokenMint: params.tokenMint,
            amount: params.amount,
            displayAmount: `${(params.amount / Math.pow(10, decimals)).toLocaleString()} tokens`
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildOfferVaultDepositIx({
            amount: params.amount,
            tokenMint: params.tokenMint,
            tokenProgram: TOKEN_PROGRAM_ID,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Vault Deposit Transaction",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeWithdraw(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(vaultParams, opts, config, opts.noInteractive);

        // Determine decimals based on known mints
        let decimals = 9;
        if (params.tokenMint.equals(config.mints.usdc)) {
            decimals = 6;
        }

        printParamSummary("Withdrawing from vault:", {
            tokenMint: params.tokenMint,
            amount: params.amount,
            displayAmount: `${(params.amount / Math.pow(10, decimals)).toLocaleString()} tokens`
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildOfferVaultWithdrawIx({
            amount: params.amount,
            tokenMint: params.tokenMint,
            tokenProgram: TOKEN_PROGRAM_ID,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Vault Withdraw Transaction",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}
