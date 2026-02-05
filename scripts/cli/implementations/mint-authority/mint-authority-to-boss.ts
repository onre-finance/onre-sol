import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { mintParams } from "../../params";
import { getTokenProgramId } from "../../utils/token-utils";

/**
 * Execute mint-authority to-boss command
 */
export async function executeMintAuthorityToBoss(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, mintParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();

                // Determine the correct token program for the mint
                const tokenProgram = getTokenProgramId(params.mint);

                return helper.buildTransferMintAuthorityToBossIx({
                    mint: params.mint,
                    tokenProgram,
                    boss,
                });
            },
            title: "Transfer Mint Authority to Boss",
            description: `Transfers mint authority of ${params.mint.toBase58().slice(0, 12)}... back to boss`,
            showParamSummary: {
                title: "Transferring mint authority to boss:",
                params: {
                    mint: params.mint,
                    targetAuthority: "boss",
                },
            },
        });
    });
}
