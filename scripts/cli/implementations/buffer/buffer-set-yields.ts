import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { bufferGrossYieldParams } from "../../params";

export async function executeBufferSetYields(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, bufferGrossYieldParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildSetBufferGrossYieldIx({
                    boss,
                    grossYield: params.grossYield,
                });
            },
            title: "Set BUFFER Gross Yield Transaction",
            description: "Updates BUFFER gross yield. Current APR is sourced from the main offer during accrual",
            showParamSummary: {
                title: "Setting BUFFER gross yield:",
                params: {
                    grossYield: params.grossYield,
                },
            },
        });
    });
}
