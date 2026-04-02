import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { setFeeDestinationParams } from "../../params";

/**
 * Execute redemption set-fee-destination command
 */
export async function executeRedemptionSetFeeDestination(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, setFeeDestinationParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();

                return helper.buildSetRedemptionFeeDestinationIx({
                    feeDestination: params.feeDestination,
                    boss,
                });
            },
            title: "Set Redemption Fee Destination Transaction",
            description: `Sets redemption fee destination to ${params.feeDestination.toBase58()}`,
            showParamSummary: {
                title: "Setting redemption fee destination:",
                params: {
                    destination: params.feeDestination,
                },
            },
        });
    });
}
