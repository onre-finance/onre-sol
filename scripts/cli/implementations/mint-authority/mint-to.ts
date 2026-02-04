import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { mintParams } from "../../params";

/**
 * Execute mint-to command
 */
export async function executeMintTo(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, mintParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                return helper.buildMintToIx({
                    amount: params.amount,
                });
            },
            title: "Mint To Transaction",
            description: `Mints ${params.amount} tokens of ${params.mint.toBase58().slice(0, 12)}... to boss`,
            showParamSummary: {
                title: "Minting tokens to boss:",
                params: {
                    mint: params.mint,
                    amount: params.amount,
                },
            },
        });
    });
}
