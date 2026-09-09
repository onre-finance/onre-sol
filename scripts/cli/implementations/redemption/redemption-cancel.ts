import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { requestParams } from "../../params";
import { getTokenProgramId } from "../../utils/token-utils";

/**
 * Execute redemption cancel command
 */
export async function executeRedemptionCancel(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, requestParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const requester = helper.wallet.publicKey;
                const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);
                const redemptionRequestPda = helper.getRedemptionRequestPda(redemptionOfferPda, params.redeemer, params.requestId);

                return helper.buildCancelRedemptionRequestIx({
                    redemptionOfferPda,
                    redemptionRequestPda,
                    signer: requester,
                    tokenInMint: params.tokenIn,
                    tokenProgram: getTokenProgramId(params.tokenIn),
                });
            },
            title: "Cancel Redemption Request Transaction",
            description: `Cancels redemption request ${params.requestId}`,
            payer: context.helper.wallet.publicKey,
            showParamSummary: {
                title: "Cancelling redemption request:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    requestId: params.requestId,
                    redeemer: params.redeemer,
                },
            },
        });
    });
}
