import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { cacheYieldsParams } from "../../params";

export async function executeCacheSetYields(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, cacheYieldsParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildSetCacheYieldsIx({
                    boss,
                    grossYield: params.grossYield,
                    currentYield: params.currentYield,
                });
            },
            title: "Set CACHE Yields Transaction",
            description: "Updates CACHE gross/current yield parameters",
            showParamSummary: {
                title: "Setting CACHE yields:",
                params: {
                    grossYield: params.grossYield,
                    currentYield: params.currentYield,
                },
            },
        });
    });
}
