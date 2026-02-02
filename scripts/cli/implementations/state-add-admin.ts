import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction } from "../helpers";
import { adminParams } from "../params/state";

/**
 * Execute state add-admin command
 */
export async function executeStateAddAdmin(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, adminParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildAddAdminIx({
                    admin: params.admin,
                    boss
                });
            },
            title: "Add Admin Transaction",
            description: `Adds ${params.admin.toBase58()} as admin`,
            showParamSummary: {
                title: "Adding admin:",
                params: {
                    admin: params.admin
                }
            }
        });
    });
}
