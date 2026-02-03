import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction } from "../helpers";
import { mintParams } from "../params/mint-authority";

/**
 * Execute mint-authority to-boss command
 */
export async function executeMintAuthorityToBoss(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, mintParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildTransferMintAuthorityToBossIx({
                    mint: params.mint,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    boss
                });
            },
            title: "Transfer Mint Authority to Boss",
            description: `Transfers mint authority of ${params.mint.toBase58().slice(0, 12)}... back to boss`,
            showParamSummary: {
                title: "Transferring mint authority to boss:",
                params: {
                    mint: params.mint,
                    targetAuthority: "boss"
                }
            }
        });
    });
}
