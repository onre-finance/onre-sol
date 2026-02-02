import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction } from "../helpers";
import { redemptionOfferParams } from "../params/redemption";

/**
 * Execute redemption make-offer command
 */
export async function executeRedemptionMakeOffer(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, redemptionOfferParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildMakeRedemptionOfferIx({
                    tokenInMint: params.tokenIn,
                    tokenOutMint: params.tokenOut,
                    tokenInProgram: TOKEN_PROGRAM_ID,
                    tokenOutProgram: TOKEN_PROGRAM_ID,
                    feeBasisPoints: params.fee,
                    boss
                });
            },
            title: "Make Redemption Offer Transaction",
            description: "Creates redemption offer",
            showParamSummary: {
                title: "Creating redemption offer:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    fee: `${params.fee / 100}% (${params.fee} bps)`
                }
            }
        });
    });
}
