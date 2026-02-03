import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction } from "../helpers";
import { redemptionAdminParams } from "../params/state";

/**
 * Execute state set-redemption-admin command
 */
export async function executeStateSetRedemptionAdmin(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, redemptionAdminParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildSetRedemptionAdminIx({
                    redemptionAdmin: params.redemptionAdmin,
                    boss
                });
            },
            title: "Set Redemption Admin Transaction",
            description: `Sets redemption admin to ${params.redemptionAdmin.toBase58()}`,
            showParamSummary: {
                title: "Setting redemption admin:",
                params: {
                    redemptionAdmin: params.redemptionAdmin
                }
            }
        });
    });
}
