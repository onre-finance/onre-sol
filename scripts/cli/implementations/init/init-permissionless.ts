import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { initPermissionlessParams } from "../../params";

/**
 * Execute init permissionless command
 */
export async function executeInitPermissionless(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, initPermissionlessParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildInitializePermissionlessAuthorityIx({
                    name: params.name,
                    boss
                });
            },
            title: "Initialize Permissionless Authority Transaction",
            description: `Creates permissionless vault authority with name: ${params.name}`,
            showParamSummary: {
                title: "Initializing permissionless authority:",
                params: {
                    name: params.name
                }
            }
        });
    });
}
