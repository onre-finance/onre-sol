import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { createRequestParams } from "../../params";

/**
 * Execute redemption create-request command
 */
export async function executeRedemptionCreateRequest(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, createRequestParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);
                const redeemer = helper.wallet.publicKey;

                return helper.buildCreateRedemptionRequestIx({
                    redemptionOfferPda,
                    tokenInMint: params.tokenIn,
                    amount: params.amount,
                    redeemer,
                });
            },
            title: "Create Redemption Request Transaction",
            description: `Creates redemption request for ${params.amount} tokens`,
            payer: context.helper.wallet.publicKey,
            showParamSummary: {
                title: "Creating redemption request:",
                params: {
                    redeemer: context.helper.wallet.publicKey,
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    amount: params.amount,
                },
            },
        });
    });
}
