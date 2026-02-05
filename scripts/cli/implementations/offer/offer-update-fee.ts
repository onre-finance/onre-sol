import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { updateFeeParams } from "../../params";

/**
 * Execute offer update-fee command
 */
export async function executeOfferUpdateFee(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, updateFeeParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildUpdateOfferFeeIx({
                    tokenInMint: params.tokenIn,
                    tokenOutMint: params.tokenOut,
                    newFeeBasisPoints: params.fee,
                    boss,
                });
            },
            title: "Update Offer Fee Transaction",
            description: `Updates fee to ${params.fee / 100}%`,
            showParamSummary: {
                title: "Updating offer fee:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    newFee: `${params.fee / 100}% (${params.fee} bps)`,
                },
            },
        });
    });
}
