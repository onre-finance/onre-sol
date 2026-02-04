import chalk from "chalk";
import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, confirmDangerousOperation, executeCommand } from "../../helpers";

/**
 * Execute state clear-admins command
 */
export async function executeStateClearAdmins(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        const { helper } = context;

        // Get current state to show admin count
        const state = await helper.program.account.state.fetch(helper.statePda);
        const adminCount = state.admins.filter((a: any) => !a.equals(helper.program.provider.publicKey)).length;

        if (adminCount === 0 && !opts.json) {
            console.log(chalk.yellow("No admins to clear."));
            return;
        }

        // Confirm dangerous action
        if (!opts.json && !opts.dryRun) {
            console.log(chalk.yellow(`\nThis will remove all ${adminCount} admin(s).`));

            const confirmed = await confirmDangerousOperation(
                `Remove all ${adminCount} admin(s)?`
            );

            if (!confirmed) {
                console.log(chalk.yellow("\nOperation cancelled."));
                return;
            }
        }

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildClearAdminsIx({
                    boss
                });
            },
            title: "Clear Admins Transaction",
            description: `Removes all ${adminCount} admin(s)`
        });
    });
}
