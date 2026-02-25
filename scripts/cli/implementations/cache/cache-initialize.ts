import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { cacheInitParams } from "../../params";

export async function executeCacheInitialize(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, cacheInitParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildInitializeCacheIx({
                    boss,
                    cacheAdmin: params.cacheAdmin,
                    onycMint: params.onycMint,
                });
            },
            title: "Initialize CACHE Transaction",
            description: "Initializes CACHE state and vault authority accounts",
            showParamSummary: {
                title: "Initializing CACHE:",
                params: {
                    cacheAdmin: params.cacheAdmin,
                    onycMint: params.onycMint,
                },
            },
        });
    });
}
