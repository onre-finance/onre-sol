import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction } from "../helpers";
import { requestParams } from "../params/redemption";

/**
 * Execute redemption fulfill command
 */
export async function executeRedemptionFulfill(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, requestParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const state = await helper.getState();
                const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);
                const redemptionRequestPda = helper.getRedemptionRequestPda(redemptionOfferPda, params.requestId);

                return helper.buildFulfillRedemptionRequestIx({
                    redemptionOfferPda,
                    redemptionRequestPda,
                    redemptionAdmin: state.redemptionAdmin
                });
            },
            title: "Fulfill Redemption Request Transaction",
            description: `Fulfills redemption request #${params.requestId}`,
            showParamSummary: {
                title: "Fulfilling redemption request:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    requestId: params.requestId
                }
            }
        });
    });
}
