import type { GlobalOptions } from "../prompts";
import { buildAndHandleTransaction, executeCommand } from "../helpers";
import { deleteVectorParams } from "../params/offer";

/**
 * Execute offer delete-vector command
 */
export async function executeOfferDeleteVector(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, deleteVectorParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildDeleteOfferVectorIx({
                    tokenInMint: params.tokenIn,
                    tokenOutMint: params.tokenOut,
                    vectorStartTimestamp: params.startTime,
                    boss
                });
            },
            title: "Delete Offer Vector Transaction",
            description: `Deletes vector starting at ${params.startTime} from offer`,
            showParamSummary: {
                title: "Deleting price vector:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    startTime: new Date(params.startTime * 1000).toISOString()
                }
            }
        });
    });
}
