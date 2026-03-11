import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { cacheGrossYieldParams } from "../../params";

export async function executeCacheSetYields(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, cacheGrossYieldParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildSetCacheGrossYieldIx({
                    boss,
                    grossYield: params.grossYield,
                });
            },
            title: "Set CACHE Gross Yield Transaction",
            description: "Updates CACHE gross yield. Current APR is sourced from the main offer during accrual",
            showParamSummary: {
                title: "Setting CACHE gross yield:",
                params: {
                    grossYield: params.grossYield,
                },
            },
        });
    });
}
