import BN from "bn.js";
import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { fulfillRequestParams } from "../../params";
import { PublicKey } from "@solana/web3.js";

/**
 * Execute redemption fulfill command
 *
 * Supports partial fulfillment: pass --amount to fulfill only part of the request.
 * Omit --amount to fulfill the full remaining balance.
 */
export async function executeRedemptionFulfill(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, fulfillRequestParams, async (context) => {
        const { helper, params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async () => {
                const state = await helper.getState();
                const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);
                const redemptionRequestPda = helper.getRedemptionRequestPda(redemptionOfferPda, params.requestId);

                // Validate that redemption_admin is set
                if (!state.redemptionAdmin || state.redemptionAdmin.equals(PublicKey.default)) {
                    throw new Error(
                        "Redemption admin is not set in program state. " +
                            "Please set a redemption admin first using: npm run cli -- state set-redemption-admin",
                    );
                }

                // Determine the amount to fulfill
                let fulfillAmount: BN;
                if (params.amount != null) {
                    fulfillAmount = new BN(params.amount.toString());
                } else {
                    // Default: fulfill all remaining (amount - fulfilled_amount)
                    const request = await helper.program.account.redemptionRequest.fetch(redemptionRequestPda);
                    fulfillAmount = (request.amount as BN).sub(request.fulfilledAmount as BN);
                    if (fulfillAmount.lten(0)) {
                        throw new Error("Redemption request is already fully fulfilled.");
                    }
                }

                return helper.buildFulfillRedemptionRequestIx({
                    redemptionOfferPda,
                    redemptionRequestPda,
                    redemptionAdmin: state.redemptionAdmin,
                    amount: fulfillAmount,
                });
            },
            title: "Fulfill Redemption Request Transaction",
            description: `Fulfills redemption request #${params.requestId}${params.amount != null ? ` (partial: ${params.amount})` : " (full remaining)"}`,
            showParamSummary: {
                title: "Fulfilling redemption request:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    requestId: params.requestId,
                    ...(params.amount != null ? { amount: params.amount } : { amount: "(full remaining)" }),
                },
            },
        });
    });
}
