import chalk from "chalk";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction, confirmDangerousOperation } from "../helpers";
import { transferMintAuthorityParams } from "../params/mint-authority";

/**
 * Execute mint-authority to-program command
 */
export async function executeMintAuthorityToProgram(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, transferMintAuthorityParams, async (context) => {
        const { helper, params } = context;

        // Show transfer details
        if (!opts.json) {
            console.log(chalk.gray("\nTransferring mint authority:"));
            console.log(`  Mint:              ${params.mint.toBase58()}`);
            console.log(`  Target Authority:  ${helper.pdas.mintAuthorityPda.toBase58()}`);
            console.log();
        }

        // Confirm dangerous action
        const confirmed = await confirmDangerousOperation(
            chalk.yellow("This will generate a transaction to transfer mint authority to the program. Continue?")
        );

        if (!confirmed) {
            console.log(chalk.yellow("\nOperation cancelled."));
            return;
        }

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildTransferMintAuthorityToProgramIx({
                    mint: params.mint,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    boss
                });
            },
            title: "Transfer Mint Authority to Program",
            description: `Transfers mint authority of ${params.mint.toBase58().slice(0, 12)}... to program PDA`
        });
    });
}
