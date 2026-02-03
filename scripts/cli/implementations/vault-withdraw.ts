import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import type { GlobalOptions } from "../prompts";
import { executeCommand, buildAndHandleTransaction } from "../helpers";
import { vaultParams } from "../params";
import { getTokenDecimals } from "../utils/token-utils";

/**
 * Execute vault withdraw command
 */
export async function executeVaultWithdraw(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, vaultParams, async (context) => {
        const { params } = context;

        // Determine decimals for display
        const decimals = getTokenDecimals(params.tokenMint);

        await buildAndHandleTransaction(context, {
            buildIx: async (helper, params) => {
                const boss = await helper.getBoss();
                return helper.buildOfferVaultWithdrawIx({
                    amount: params.amount,
                    tokenMint: params.tokenMint,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    boss
                });
            },
            title: "Vault Withdraw Transaction",
            showParamSummary: {
                title: "Withdrawing from vault:",
                params: {
                    tokenMint: params.tokenMint,
                    amount: params.amount,
                    displayAmount: `${(params.amount / Math.pow(10, decimals)).toLocaleString()} tokens`
                }
            }
        });
    });
}
