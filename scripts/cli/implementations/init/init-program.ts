import chalk from "chalk";
import { PublicKey } from "@solana/web3.js";
import { config } from "../../../utils/script-helper";
import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, confirmDangerousOperation, executeCommand } from "../../helpers";
import { initProgramParams } from "../../params";

/**
 * Execute init program command
 */
export async function executeInitProgram(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, initProgramParams, async (context) => {
        const { params } = context;

        // Get program data PDA
        const [programDataPda] = PublicKey.findProgramAddressSync(
            [config.programId.toBuffer()],
            new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111")
        );

        // Show what will be initialized
        if (!opts.json) {
            console.log(chalk.gray("\nProgram initialization details:"));
            console.log(`  Program ID:    ${config.programId.toBase58()}`);
            console.log(`  ONyc Mint:     ${params.onycMint.toBase58()}`);
            console.log(`  Program Data:  ${programDataPda.toBase58()}`);
            console.log(`  Boss:          ${config.boss.toBase58()}`);
            console.log();
        }

        // Confirm action (skip in dry-run mode)
        if (!opts.dryRun) {
            const confirmed = await confirmDangerousOperation(
                chalk.yellow("This will initialize the program state. Continue?")
            );

            if (!confirmed) {
                console.log(chalk.yellow("\nOperation cancelled."));
                return;
            }
        }

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                return helper.buildInitializeIx({
                    boss: config.boss,
                    programData: programDataPda,
                    onycMint: params.onycMint
                });
            },
            title: "Initialize Program Transaction",
            description: "Initializes the program state account",
            payer: config.boss
        });
    });
}
