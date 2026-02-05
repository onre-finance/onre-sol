import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { requestParams } from "../../params";

/**
 * Execute redemption cancel command
 */
export async function executeRedemptionCancel(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, requestParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const requester = await helper.getBoss();
                const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);
                const redemptionRequestPda = helper.getRedemptionRequestPda(redemptionOfferPda, params.requestId);

                return helper.buildCancelRedemptionRequestIx({
                    redemptionOfferPda,
                    redemptionRequestPda,
                    signer: requester,
                });
            },
            title: "Cancel Redemption Request Transaction",
            description: `Cancels redemption request #${params.requestId}`,
            showParamSummary: {
                title: "Cancelling redemption request:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    requestId: params.requestId,
                },
            },
        });
    });
}
