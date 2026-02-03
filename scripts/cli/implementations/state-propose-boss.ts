import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction } from "../helpers";
import { proposeBossParams } from "../params/state";

/**
 * Execute state propose-boss command
 */
export async function executeStateProposeBoss(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, proposeBossParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildProposeBossIx({
                    newBoss: params.newBoss,
                    boss
                });
            },
            title: "Propose Boss Transfer Transaction",
            description: `Proposes ${params.newBoss.toBase58()} as new boss`,
            showParamSummary: {
                title: "Proposing new boss:",
                params: {
                    newBoss: params.newBoss
                }
            }
        });
    });
}
