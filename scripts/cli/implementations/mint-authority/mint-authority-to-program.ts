import chalk from "chalk";
import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, confirmDangerousOperation, executeCommand } from "../../helpers";
import { transferMintAuthorityParams } from "../../params";
import { getTokenProgramId } from "../../utils/token-utils";

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

        // Confirm dangerous action (skip in dry-run mode)
        if (!opts.dryRun) {
            const confirmed = await confirmDangerousOperation(
                chalk.yellow("This will generate a transaction to transfer mint authority to the program. Continue?")
            );

            if (!confirmed) {
                console.log(chalk.yellow("\nOperation cancelled."));
                return;
            }
        }

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();

                // Determine the correct token program for the mint
                const tokenProgram = getTokenProgramId(params.mint);

                return helper.buildTransferMintAuthorityToProgramIx({
                    mint: params.mint,
                    tokenProgram,
                    boss
                });
            },
            title: "Transfer Mint Authority to Program",
            description: `Transfers mint authority of ${params.mint.toBase58().slice(0, 12)}... to program PDA`
        });
    });
}
