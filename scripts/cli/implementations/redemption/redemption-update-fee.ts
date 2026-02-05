import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { updateRedemptionFeeParams } from "../../params";

/**
 * Execute redemption update-fee command
 */
export async function executeRedemptionUpdateFee(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, updateRedemptionFeeParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);

                return helper.buildUpdateRedemptionOfferFeeIx({
                    redemptionOfferPda,
                    newFeeBasisPoints: params.fee,
                    boss,
                });
            },
            title: "Update Redemption Fee Transaction",
            description: `Updates redemption fee to ${params.fee / 100}%`,
            showParamSummary: {
                title: "Updating redemption fee:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    newFee: `${params.fee / 100}% (${params.fee} bps)`,
                },
            },
        });
    });
}
