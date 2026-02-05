import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { makeOfferParams } from "../../params";
import { getTokenProgramId } from "../../utils/token-utils";

/**
 * Execute offer make command
 */
export async function executeOfferMake(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, makeOfferParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                const instructions = [];

                // Determine the correct token programs for each mint
                const tokenInProgram = getTokenProgramId(params.tokenIn);
                const tokenOutProgram = getTokenProgramId(params.tokenOut);

                // Log token programs for debugging
                if (!opts.json) {
                    console.log(`  Token In Program: ${tokenInProgram.toBase58().slice(0, 12)}...`);
                    console.log(`  Token Out Program: ${tokenOutProgram.toBase58().slice(0, 12)}...`);
                }

                // Add permissionless account creation if needed
                if (params.permissionless) {
                    const permissionlessIxs = await helper.buildCreatePermissionlessTokenAccountsIxs({
                        tokenInMint: params.tokenIn,
                        tokenOutMint: params.tokenOut,
                        tokenInProgram,
                        tokenOutProgram,
                        payer: boss,
                    });
                    instructions.push(...permissionlessIxs);
                }

                // Add main instruction
                const makeOfferIx = await helper.buildMakeOfferIx({
                    tokenInMint: params.tokenIn,
                    tokenOutMint: params.tokenOut,
                    tokenInProgram,
                    feeBasisPoints: params.fee,
                    needsApproval: params.needsApproval,
                    allowPermissionless: params.permissionless,
                    boss,
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
                    permissionless: params.permissionless,
                },
            },
        });
    });
}
