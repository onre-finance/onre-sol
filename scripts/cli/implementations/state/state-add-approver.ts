import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { approverParams } from "../../params";

/**
 * Execute state add-approver command
 */
export async function executeStateAddApprover(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, approverParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildAddApproverIx({
                    approver: params.approver,
                    boss
                });
            },
            title: "Add Approver Transaction",
            description: `Adds ${params.approver.toBase58()} as approver`,
            showParamSummary: {
                title: "Adding approver:",
                params: {
                    approver: params.approver
                }
            }
        });
    });
}
