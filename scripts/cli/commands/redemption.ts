import { Command } from "commander";
import chalk from "chalk";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { config, NetworkConfig, ScriptHelper } from "../../utils/script-helper";
import type { GlobalOptions } from "../prompts";
import { ParamDefinition, promptForParams } from "../prompts";
import { handleTransaction } from "../transaction/handler";
import { printNetworkBanner, printParamSummary } from "../utils/display";

const redemptionOfferParams: ParamDefinition[] = [
    {
        name: "tokenIn",
        type: "mint",
        description: "Token in mint (ONyc for redemptions)",
        required: true,
        flag: "--token-in",
        shortFlag: "-i",
        default: (cfg: NetworkConfig) => cfg.mints.onyc
    },
    {
        name: "tokenOut",
        type: "mint",
        description: "Token out mint (USDC for redemptions)",
        required: true,
        flag: "--token-out",
        shortFlag: "-o",
        default: (cfg: NetworkConfig) => cfg.mints.usdc
    },
    {
        name: "fee",
        type: "basisPoints",
        description: "Redemption fee in basis points",
        required: false,
        flag: "--fee",
        shortFlag: "-f",
        default: 0
    }
];

/**
 * Register redemption subcommands
 */
export function registerRedemptionCommands(program: Command): void {
    // redemption make-offer
    program
        .command("make-offer")
        .description("Create a redemption offer (ONyc -> USDC)")
        .option("-i, --token-in <mint>", "Token in mint (ONyc)")
        .option("-o, --token-out <mint>", "Token out mint (USDC)")
        .option("-f, --fee <bps>", "Fee in basis points")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeMakeRedemptionOffer(opts);
        });
}

// === Command Implementations ===

async function executeMakeRedemptionOffer(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(redemptionOfferParams, opts, config, opts.noInteractive);

        printParamSummary("Creating redemption offer:", {
            tokenIn: params.tokenIn,
            tokenOut: params.tokenOut,
            fee: `${params.fee / 100}% (${params.fee} bps)`
        });

        const boss = await helper.getBoss();
        const ix = await helper.buildMakeRedemptionOfferIx({
            tokenInMint: params.tokenIn,
            tokenInProgram: TOKEN_PROGRAM_ID,
            tokenOutMint: params.tokenOut,
            tokenOutProgram: TOKEN_PROGRAM_ID,
            feeBasisPoints: params.fee,
            boss
        });
        const tx = await helper.prepareTransaction({ ix, payer: boss });

        await handleTransaction(tx, helper, {
            title: "Make Redemption Offer Transaction",
            description: "Creates a redemption offer allowing ONyc holders to redeem for USDC",
            dryRun: opts.dryRun,
            json: opts.json
        });
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}
