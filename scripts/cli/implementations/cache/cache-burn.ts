import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { cacheBurnParams } from "../../params";

export async function executeCacheBurn(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, cacheBurnParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildBurnForNavIncreaseIx({
                    boss,
                    tokenInMint: params.tokenIn,
                    onycMint: params.onycMint,
                    assetAdjustmentAmount: params.assetAdjustmentAmount,
                    targetNav: params.targetNav,
                });
            },
            title: "Burn For NAV Increase Transaction",
            description: "Burns ONyc from CACHE vault to support NAV adjustment",
            showParamSummary: {
                title: "Burning from CACHE:",
                params: {
                    tokenInMint: params.tokenIn,
                    onycMint: params.onycMint,
                    assetAdjustmentAmount: params.assetAdjustmentAmount,
                    targetNav: params.targetNav,
                },
            },
        });
    });
}
