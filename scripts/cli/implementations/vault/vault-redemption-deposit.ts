import { select } from "@inquirer/prompts";
import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { vaultParams } from "../../params";
import { getTokenDecimals, getTokenProgramId } from "../../utils/token-utils";

/**
 * Execute redemption vault deposit command
 */
export async function executeVaultRedemptionDeposit(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, vaultParams, async (context) => {
        const { helper, params } = context;

        // Determine decimals for display
        const decimals = getTokenDecimals(params.tokenMint);

        // Ask who will sign the deposit (before building the instruction, since it
        // affects the depositor account in the instruction and the transaction payer)
        const depositorMode = opts.noInteractive
            ? "local"
            : await select({
                  message: "Who will be the depositor?",
                  choices: [
                      { name: "Local wallet (sign and send directly)", value: "local" },
                      { name: "Boss / Squad multisig (copy Base58)", value: "boss" },
                  ],
              });

        const depositor = depositorMode === "boss"
            ? await helper.getBoss()
            : helper.wallet.publicKey;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper, params) => {
                const tokenProgram = getTokenProgramId(params.tokenMint);

                return helper.buildRedemptionVaultDepositIx({
                    amount: params.amount,
                    tokenMint: params.tokenMint,
                    tokenProgram,
                    depositor,
                });
            },
            payer: depositor,
            title: "Redemption Vault Deposit Transaction",
            showParamSummary: {
                title: "Depositing to redemption vault:",
                params: {
                    tokenMint: params.tokenMint,
                    amount: params.amount,
                    depositor: depositor.toBase58(),
                    displayAmount: `${(params.amount / Math.pow(10, decimals)).toLocaleString()} tokens`,
                },
            },
        });
    });
}