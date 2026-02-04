import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { takeOfferParams } from "../../params";
import { getTokenProgramId } from "../../utils/token-utils";

/**
 * Execute offer take command
 */
export async function executeOfferTake(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, takeOfferParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                // Use the wallet's public key as the user (the person taking the offer)
                const user = helper.wallet.publicKey;

                // Determine the correct token programs for each mint
                const tokenInProgram = getTokenProgramId(params.tokenIn);
                const tokenOutProgram = getTokenProgramId(params.tokenOut);

                // Choose between regular and permissionless flow
                if (params.permissionless) {
                    return await helper.buildTakeOfferPermissionlessIx({
                        tokenInAmount: params.amount,
                        tokenInMint: params.tokenIn,
                        tokenOutMint: params.tokenOut,
                        user,
                        tokenInProgram,
                        tokenOutProgram,
                    });
                } else {
                    return await helper.buildTakeOfferIx({
                        tokenInAmount: params.amount,
                        tokenInMint: params.tokenIn,
                        tokenOutMint: params.tokenOut,
                        user,
                        tokenInProgram,
                        tokenOutProgram,
                    });
                }
            },
            title: "Take Offer Transaction",
            description: `Takes ${params.amount} of ${params.tokenIn.toBase58().slice(0, 8)}... for ${params.tokenOut.toBase58().slice(0, 8)}...`,
            payer: context.helper.wallet.publicKey,
            showParamSummary: {
                title: "Taking offer:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    amount: params.amount,
                    permissionless: params.permissionless,
                },
            },
        });
    });
}
