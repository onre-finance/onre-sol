import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { bufferInitParams } from "../../params";

export async function executeBufferInitialize(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, bufferInitParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildInitializeBufferIx({
                    boss,
                    bufferAdmin: params.bufferAdmin,
                    offer: params.offer,
                    onycMint: params.onycMint,
                });
            },
            title: "Initialize BUFFER Transaction",
            description: "Initializes BUFFER state and vault authority accounts",
            showParamSummary: {
                title: "Initializing BUFFER:",
                params: {
                    bufferAdmin: params.bufferAdmin,
                    offer: params.offer,
                    onycMint: params.onycMint,
                },
            },
        });
    });
}
