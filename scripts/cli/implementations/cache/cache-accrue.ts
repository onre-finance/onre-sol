import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { cacheOnycMintParam } from "../../params";

export async function executeCacheAccrue(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, cacheOnycMintParam, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                return helper.buildAccrueCacheIx({
                    onycMint: params.onycMint,
                    cacheAdmin: helper.wallet.publicKey,
                });
            },
            title: "Accrue CACHE Transaction",
            description: "Accrues CACHE spread and mints ONyc to CACHE vault",
            showParamSummary: {
                title: "Accruing CACHE:",
                params: {
                    onycMint: params.onycMint,
                    signer: "Current CLI wallet must be cache_admin",
                },
            },
        });
    });
}
