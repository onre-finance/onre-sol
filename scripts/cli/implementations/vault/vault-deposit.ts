import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { vaultParams } from "../../params";
import { getTokenDecimals, getTokenProgramId } from "../../utils/token-utils";

/**
 * Execute vault deposit command
 */
export async function executeVaultDeposit(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, vaultParams, async (context) => {
        const { params } = context;

        // Determine decimals for display
        const decimals = getTokenDecimals(params.tokenMint);

        await buildAndHandleTransaction(context, {
            buildIx: async (helper, params) => {
                const boss = await helper.getBoss();

                // Determine the correct token program for the mint
                const tokenProgram = getTokenProgramId(params.tokenMint);

                return helper.buildOfferVaultDepositIx({
                    amount: params.amount,
                    tokenMint: params.tokenMint,
                    tokenProgram,
                    boss
                });
            },
            title: "Vault Deposit Transaction",
            showParamSummary: {
                title: "Depositing to vault:",
                params: {
                    tokenMint: params.tokenMint,
                    amount: params.amount,
                    displayAmount: `${(params.amount / Math.pow(10, decimals)).toLocaleString()} tokens`
                }
            }
        });
    });
}
