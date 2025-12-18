import { Command } from "commander";
import chalk from "chalk";
import { PublicKey } from "@solana/web3.js";
import { config, NetworkConfig, ScriptHelper } from "../../utils/script-helper";
import type { GlobalOptions } from "../prompts";
import { ParamDefinition, promptForParams } from "../prompts";
import {
    printApy,
    printCirculatingSupply,
    printNav,
    printNavAdjustment,
    printNetworkBanner,
    printTvl
} from "../utils/display";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Common params for market queries
// Note: param names match Commander's camelCase conversion (--token-in -> tokenIn)
const offerParams: ParamDefinition[] = [
    {
        name: "tokenIn",
        type: "mint",
        description: "Token in mint (typically USDC)",
        required: true,
        flag: "--token-in",
        shortFlag: "-i",
        default: (cfg: NetworkConfig) => cfg.mints.usdc
    },
    {
        name: "tokenOut",
        type: "mint",
        description: "Token out mint (typically ONyc)",
        required: true,
        flag: "--token-out",
        shortFlag: "-o",
        default: (cfg: NetworkConfig) => cfg.mints.onyc
    }
];

/**
 * Register market subcommands
 */
export function registerMarketCommands(program: Command): void {
    // market nav
    program
        .command("nav")
        .description("Get current NAV (Net Asset Value) for an offer")
        .option("-i, --token-in <mint>", "Token in mint (usdc, onyc, usdg, or address)")
        .option("-o, --token-out <mint>", "Token out mint (usdc, onyc, usdg, or address)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeGetNav(opts);
        });

    // market nav-adjustment
    program
        .command("nav-adjustment")
        .description("Get NAV adjustment (price change) for an offer")
        .option("-i, --token-in <mint>", "Token in mint (usdc, onyc, usdg, or address)")
        .option("-o, --token-out <mint>", "Token out mint (usdc, onyc, usdg, or address)")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeGetNavAdjustment(opts);
        });

    // market apy
    program
        .command("apy")
        .description("Get current APY for an offer")
        .option("-i, --token-in <mint>", "Token in mint")
        .option("-o, --token-out <mint>", "Token out mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeGetApy(opts);
        });

    // market tvl
    program
        .command("tvl")
        .description("Get Total Value Locked")
        .option("-i, --token-in <mint>", "Token in mint")
        .option("-o, --token-out <mint>", "Token out mint")
        .action(async (options, cmd) => {
            const opts = { ...options, ...cmd.optsWithGlobals() } as GlobalOptions & Record<string, any>;
            await executeGetTvl(opts);
        });

    // market supply
    program
        .command("supply")
        .description("Get circulating supply of ONyc tokens")
        .action(async (_, cmd) => {
            const opts = cmd.optsWithGlobals() as GlobalOptions;
            await executeGetCirculatingSupply(opts);
        });
}

// === Command Implementations ===

async function executeGetNav(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(offerParams, opts, config, opts.noInteractive);

        // Call the view method
        const nav = await helper.program.methods
            .getNav()
            .accounts({
                tokenInMint: new PublicKey(params.tokenIn),
                tokenOutMint: new PublicKey(params.tokenOut)
            })
            .view();

        console.log("Raw nav:", nav.toNumber());
        printNav(nav.toNumber(), opts.json);
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeGetNavAdjustment(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(offerParams, opts, config, opts.noInteractive);

        // Call the view method
        const nav = await helper.program.methods
            .getNavAdjustment()
            .accounts({
                tokenInMint: new PublicKey(params.tokenIn),
                tokenOutMint: new PublicKey(params.tokenOut)
            })
            .view();

        console.log("Raw nav adjustment:", nav.toNumber());
        printNavAdjustment(nav.toNumber(), opts.json);
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeGetApy(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(offerParams, opts, config, opts.noInteractive);

        // Call the view method
        const apy = await helper.program.methods
            .getApy()
            .accounts({
                tokenInMint: new PublicKey(params.tokenIn),
                tokenOutMint: new PublicKey(params.tokenOut)
            })
            .view();

        printApy(apy.toNumber(), opts.json);
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeGetTvl(opts: GlobalOptions & Record<string, any>): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();
        const params = await promptForParams(offerParams, opts, config, opts.noInteractive);

        // Get the offer vault token account
        const { getAssociatedTokenAddressSync } = await import("@solana/spl-token");
        const offerVaultTokenOut = getAssociatedTokenAddressSync(
            params.tokenOut,
            helper.pdas.offerVaultAuthorityPda,
            true
        );

        // Call the view method
        const tvl = await helper.program.methods
            .getTvl()
            .accounts({
                tokenInMint: new PublicKey(params.tokenIn),
                tokenOutMint: new PublicKey(params.tokenOut),
                vaultTokenOutAccount: offerVaultTokenOut,
                tokenOutProgram: TOKEN_PROGRAM_ID
            })
            .view();

        printTvl(tvl.toNumber(), opts.json);
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}

async function executeGetCirculatingSupply(opts: GlobalOptions): Promise<void> {
    try {
        if (!opts.json) {
            printNetworkBanner(config);
        }

        const helper = await ScriptHelper.create();

        // Get the offer vault token account
        const { getAssociatedTokenAddressSync } = await import("@solana/spl-token");
        const onycVaultAccount = getAssociatedTokenAddressSync(
            config.mints.onyc,
            helper.pdas.offerVaultAuthorityPda,
            true
        );

        // Call the view method
        const supply = await helper.program.methods
            .getCirculatingSupply()
            .accounts({
                onycVaultAccount,
                tokenProgram: TOKEN_PROGRAM_ID
            })
            .view();

        printCirculatingSupply(supply.toNumber(), opts.json);
    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}
