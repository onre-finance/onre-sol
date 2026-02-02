import chalk from "chalk";
import type { GlobalOptions } from "../prompts";
import { buildAndHandleTransaction, confirmDangerousOperation, executeCommand } from "../helpers";

/**
 * Execute state kill-switch command
 */
export async function executeStateKillSwitch(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        // Confirm dangerous action
        if (!opts.json && !opts.dryRun) {
            console.log(chalk.red("\n⚠️  WARNING: This will enable the kill switch!"));
            console.log(chalk.yellow("This is an emergency action that pauses all program operations."));
            console.log();

            const confirmed = await confirmDangerousOperation(
                "Are you sure you want to enable the kill switch?",
                undefined,
                { requireExactMatch: "ENABLE KILL SWITCH" }
            );

            if (!confirmed) {
                console.log(chalk.yellow("\nOperation cancelled."));
                return;
            }
        }

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const admin = await helper.getBoss();
                return helper.buildSetKillSwitchIx({
                    enable: true,
                    boss: admin
                });
            },
            title: "Enable Kill Switch Transaction",
            description: "⚠️  EMERGENCY: Enables kill switch to pause all operations"
        });
    });
}
