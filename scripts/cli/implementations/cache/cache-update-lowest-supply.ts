import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { cacheOnycMintParam } from "../../params";

export async function executeCacheUpdateLowestSupply(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, cacheOnycMintParam, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) =>
                helper.buildUpdateLowestSupplyIx({
                    onycMint: params.onycMint,
                }),
            title: "Update CACHE Lowest Supply Transaction",
            description: "Updates CACHE lowest observed ONyc supply",
            showParamSummary: {
                title: "Updating CACHE lowest supply:",
                params: {
                    onycMint: params.onycMint,
                },
            },
        });
    });
}
