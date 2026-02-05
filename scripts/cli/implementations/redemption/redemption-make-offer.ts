import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { redemptionOfferParams } from "../../params";
import { getTokenProgramId } from "../../utils/token-utils";

/**
 * Execute redemption make-offer command
 */
export async function executeRedemptionMakeOffer(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, redemptionOfferParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();

                // Determine the correct token programs for each mint
                const tokenInProgram = getTokenProgramId(params.tokenIn);
                const tokenOutProgram = getTokenProgramId(params.tokenOut);

                return helper.buildMakeRedemptionOfferIx({
                    tokenInMint: params.tokenIn,
                    tokenOutMint: params.tokenOut,
                    tokenInProgram,
                    tokenOutProgram,
                    feeBasisPoints: params.fee,
                    boss,
                });
            },
            title: "Make Redemption Offer Transaction",
            description: "Creates redemption offer",
            showParamSummary: {
                title: "Creating redemption offer:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    fee: `${params.fee / 100}% (${params.fee} bps)`,
                },
            },
        });
    });
}
