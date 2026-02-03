import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction } from "../helpers";
import { setOnycMintParams } from "../params/state";

/**
 * Execute state set-onyc-mint command
 */
export async function executeStateSetOnycMint(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, setOnycMintParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildSetOnycMintIx({
                    onycMint: params.mint,
                    boss
                });
            },
            title: "Set ONyc Mint Transaction",
            description: `Sets ONyc mint to ${params.mint.toBase58()}`,
            showParamSummary: {
                title: "Setting ONyc mint:",
                params: {
                    mint: params.mint
                }
            }
        });
    });
}
