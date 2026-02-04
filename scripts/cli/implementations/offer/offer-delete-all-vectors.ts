import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { tokenPairParams } from "../../params";

/**
 * Execute offer delete-all-vectors command
 */
export async function executeOfferDeleteAllVectors(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, tokenPairParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildDeleteAllOfferVectorsIx({
                    tokenInMint: params.tokenIn,
                    tokenOutMint: params.tokenOut,
                    boss
                });
            },
            title: "Delete All Offer Vectors Transaction",
            description: `Deletes all pricing vectors from offer`,
            showParamSummary: {
                title: "Deleting all price vectors:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut
                }
            }
        });
    });
}
