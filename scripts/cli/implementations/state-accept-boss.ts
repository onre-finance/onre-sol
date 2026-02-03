import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction } from "../helpers";
import { acceptBossParams } from "../params/state";

/**
 * Execute state accept-boss command
 */
export async function executeStateAcceptBoss(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, acceptBossParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                return helper.buildAcceptBossIx({
                    newBoss: params.newBoss
                });
            },
            title: "Accept Boss Transfer Transaction",
            description: "Accepts boss transfer proposal",
            payer: params.newBoss,
            showParamSummary: {
                title: "Accepting boss transfer:",
                params: {
                    newBoss: params.newBoss
                }
            }
        });
    });
}
