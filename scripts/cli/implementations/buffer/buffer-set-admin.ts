import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { bufferAdminParam } from "../../params";

export async function executeBufferSetAdmin(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, bufferAdminParam, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildSetBufferAdminIx({
                    boss,
                    bufferAdmin: params.bufferAdmin,
                });
            },
            title: "Set BUFFER Admin Transaction",
            description: `Sets BUFFER admin to ${params.bufferAdmin.toBase58()}`,
            showParamSummary: {
                title: "Setting BUFFER admin:",
                params: {
                    bufferAdmin: params.bufferAdmin,
                },
            },
        });
    });
}
