import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction } from "../helpers";
import { makeOfferParams } from "../params/offer";

/**
 * Execute offer make command
 */
export async function executeOfferMake(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, makeOfferParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                const instructions = [];

                // Add permissionless account creation if needed
                if (params.permissionless) {
                    const permissionlessIxs = await helper.buildCreatePermissionlessTokenAccountsIxs({
                        tokenInMint: params.tokenIn,
                        tokenOutMint: params.tokenOut,
                        tokenInProgram: TOKEN_PROGRAM_ID,
                        tokenOutProgram: TOKEN_PROGRAM_ID,
                        payer: boss
                    });
                    instructions.push(...permissionlessIxs);
                }

                // Add main instruction
                const makeOfferIx = await helper.buildMakeOfferIx({
                    tokenInMint: params.tokenIn,
                    tokenOutMint: params.tokenOut,
                    feeBasisPoints: params.fee,
                    needsApproval: params.needsApproval,
                    allowPermissionless: params.permissionless,
                    boss
                });
                instructions.push(makeOfferIx);

                return instructions;
            },
            title: "Make Offer Transaction",
            description: `Creates a new ${params.tokenIn.toBase58().slice(0, 8)}... → ${params.tokenOut.toBase58().slice(0, 8)}... offer`,
            showParamSummary: {
                title: "Creating offer:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    fee: `${params.fee / 100}% (${params.fee} bps)`,
                    needsApproval: params.needsApproval,
                    permissionless: params.permissionless
                }
            }
        });
    });
}
