import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { maxSupplyParams } from "../../params";

/**
 * Execute state max-supply command
 */
export async function executeStateMaxSupply(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, maxSupplyParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildConfigureMaxSupplyIx({
                    maxSupply: params.amount,
                    boss,
                });
            },
            title: "Set Max Supply Transaction",
            description: `Sets maximum supply to ${params.amount}`,
            showParamSummary: {
                title: "Setting max supply:",
                params: {
                    amount: params.amount,
                    displayAmount: `${(params.amount / 1_000_000_000).toLocaleString()} ONyc`,
                },
            },
        });
    });
}
