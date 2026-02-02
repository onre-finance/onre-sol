import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction } from "../helpers";
import { createRequestParams } from "../params/redemption";

/**
 * Execute redemption create-request command
 */
export async function executeRedemptionCreateRequest(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, createRequestParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);

                return helper.buildCreateRedemptionRequestIx({
                    redemptionOfferPda,
                    tokenInMint: params.tokenIn,
                    amount: params.amount,
                    redeemer: boss
                });
            },
            title: "Create Redemption Request Transaction",
            description: `Creates redemption request for ${params.amount} tokens`,
            showParamSummary: {
                title: "Creating redemption request:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    amount: params.amount
                }
            }
        });
    });
}
