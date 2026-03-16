import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { bufferOfferParam, bufferOnycMintParam } from "../../params";

export async function executeBufferManage(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, [...bufferOfferParam, ...bufferOnycMintParam], async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                return helper.buildManageBufferIx({
                    offer: params.offer,
                    onycMint: params.onycMint,
                });
            },
            title: "Accrue BUFFER Transaction",
            description: "Accrues BUFFER spread and mints ONyc to BUFFER vault",
            showParamSummary: {
                title: "Accruing BUFFER:",
                params: {
                    offer: params.offer,
                    onycMint: params.onycMint,
                },
            },
        });
    });
}
