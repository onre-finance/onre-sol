import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { requestParams } from "../../params";
import { PublicKey } from "@solana/web3.js";

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

                // Validate that redemption_admin is set
                if (!state.redemptionAdmin || state.redemptionAdmin.equals(PublicKey.default)) {
                    throw new Error(
                        "Redemption admin is not set in program state. " +
                        "Please set a redemption admin first using: npm run cli -- state set-redemption-admin"
                    );
                }

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
