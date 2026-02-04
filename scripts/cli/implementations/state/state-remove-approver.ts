import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { approverParams } from "../../params";

/**
 * Execute state remove-approver command
 */
export async function executeStateRemoveApprover(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, approverParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildRemoveApproverIx({
                    approver: params.approver,
                    boss
                });
            },
            title: "Remove Approver Transaction",
            description: `Removes ${params.approver.toBase58()} from approvers`,
            showParamSummary: {
                title: "Removing approver:",
                params: {
                    approver: params.approver
                }
            }
        });
    });
}
