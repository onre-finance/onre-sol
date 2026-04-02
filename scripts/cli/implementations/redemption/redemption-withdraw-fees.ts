import BN from "bn.js";
import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { withdrawFeesParams } from "../../params";
import { getTokenProgramId } from "../../utils/token-utils";

/**
 * Execute redemption withdraw-fees command
 */
export async function executeRedemptionWithdrawFees(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, withdrawFeesParams, async (context) => {
        const { params } = context;

        const tokenInProgram = getTokenProgramId(params.tokenIn);
        const amount = new BN(params.amount ?? 0);

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();

                return helper.buildWithdrawRedemptionFeesIx({
                    tokenInMint: params.tokenIn,
                    tokenInProgram,
                    destination: params.destination,
                    amount,
                    boss,
                });
            },
            title: "Withdraw Redemption Fees Transaction",
            description: `Withdraws redemption fees to ${params.destination.toBase58()}`,
            showParamSummary: {
                title: "Withdrawing redemption fees:",
                params: {
                    tokenIn: params.tokenIn,
                    destination: params.destination,
                    amount: amount.isZero() ? "full balance" : amount.toString(),
                },
            },
        });
    });
}
