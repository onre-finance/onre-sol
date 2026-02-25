import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { cacheAdminParam } from "../../params";

export async function executeCacheSetAdmin(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, cacheAdminParam, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildSetCacheAdminIx({
                    boss,
                    cacheAdmin: params.cacheAdmin,
                });
            },
            title: "Set CACHE Admin Transaction",
            description: `Sets CACHE admin to ${params.cacheAdmin.toBase58()}`,
            showParamSummary: {
                title: "Setting CACHE admin:",
                params: {
                    cacheAdmin: params.cacheAdmin,
                },
            },
        });
    });
}
