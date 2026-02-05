import chalk from "chalk";
import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, confirmDangerousOperation, executeCommand } from "../../helpers";

/**
 * Execute state close command
 */
export async function executeStateClose(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        // Confirm dangerous action
        if (!opts.json && !opts.dryRun) {
            console.log(chalk.red("\n⚠️  DANGER: This will close the program state account!"));
            console.log(chalk.yellow("This action is irreversible and will disable the entire program."));
            console.log();

            const confirmed = await confirmDangerousOperation("Type 'CLOSE STATE' to confirm:", undefined, { requireExactMatch: "CLOSE STATE" });

            if (!confirmed) {
                console.log(chalk.yellow("\nOperation cancelled."));
                return;
            }
        }

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildCloseStateIx({
                    boss,
                });
            },
            title: "Close State Transaction",
            description: "⚠️  DANGER: Closes the program state account",
        });
    });
}
