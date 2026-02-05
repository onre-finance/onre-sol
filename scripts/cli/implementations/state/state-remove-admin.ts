import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { adminParams } from "../../params";

/**
 * Execute state remove-admin command
 */
export async function executeStateRemoveAdmin(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, adminParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildRemoveAdminIx({
                    admin: params.admin,
                    boss,
                });
            },
            title: "Remove Admin Transaction",
            description: `Removes ${params.admin.toBase58()} from admins`,
            showParamSummary: {
                title: "Removing admin:",
                params: {
                    admin: params.admin,
                },
            },
        });
    });
}
