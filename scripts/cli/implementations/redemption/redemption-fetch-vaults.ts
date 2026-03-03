import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { config } from "../../../utils/script-helper";
import { getTokenProgramId } from "../../utils/token-utils";
import { printRedemptionVaults } from "../../utils/display";

/**
 * Execute redemption fetch-vaults command
 */
export async function executeRedemptionFetchVaults(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        const { helper } = context;

        // Derive the redemption vault authority PDA
        const [authority] = PublicKey.findProgramAddressSync(
            [Buffer.from("redemption_offer_vault_authority")],
            helper.program.programId,
        );

        const mints = [
            { name: "USDC", mint: config.mints.usdc },
            { name: "USDG", mint: config.mints.usdg },
            { name: "ONyc", mint: config.mints.onyc },
        ];

        const vaults = await Promise.all(
            mints.map(async ({ name, mint }) => {
                const tokenProgram = getTokenProgramId(mint);
                const ata = getAssociatedTokenAddressSync(mint, authority, true, tokenProgram);

                const accountInfo = await helper.connection.getAccountInfo(ata);
                if (!accountInfo) {
                    return { token: name, mint: mint.toBase58(), ata: ata.toBase58(), balance: null, decimals: null, initialized: false };
                }

                const tokenBalance = await helper.connection.getTokenAccountBalance(ata);
                return {
                    token: name,
                    mint: mint.toBase58(),
                    ata: ata.toBase58(),
                    balance: tokenBalance.value.uiAmountString ?? "0",
                    decimals: tokenBalance.value.decimals,
                    initialized: true,
                };
            }),
        );

        printRedemptionVaults(vaults, authority.toBase58(), opts.json);
    });
}
