import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { config } from "../../../utils/script-helper";
import { getTokenProgramId } from "../../utils/token-utils";
import { printVaultList } from "../../utils/display";

type VaultEntry = { token: string; mint: string; ata: string; balance: string | null; decimals: number | null; initialized: boolean };
type VaultGroup = { name: string; authority: string; vaults: VaultEntry[] };

/**
 * Execute vault list command
 */
export async function executeVaultList(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        const { helper } = context;

        const [redemptionAuthority] = PublicKey.findProgramAddressSync(
            [Buffer.from("redemption_offer_vault_authority")],
            helper.program.programId,
        );

        const authorities = [
            { name: "Offer Vault", pda: helper.pdas.offerVaultAuthorityPda },
            { name: "Permissionless Vault", pda: helper.pdas.permissionlessVaultAuthorityPda },
            { name: "Redemption Vault", pda: redemptionAuthority },
        ];

        const mints = [
            { name: "USDC", mint: config.mints.usdc },
            { name: "USDG", mint: config.mints.usdg },
            { name: "ONyc", mint: config.mints.onyc },
        ];

        const groups: VaultGroup[] = await Promise.all(
            authorities.map(async ({ name, pda }) => {
                const vaults = await Promise.all(
                    mints.map(async ({ name: tokenName, mint }) => {
                        const tokenProgram = getTokenProgramId(mint);
                        const ata = getAssociatedTokenAddressSync(mint, pda, true, tokenProgram);
                        const parsed = await helper.connection.getParsedAccountInfo(ata);

                        if (!parsed.value || !("parsed" in parsed.value.data)) {
                            return { token: tokenName, mint: mint.toBase58(), ata: ata.toBase58(), balance: null, decimals: null, initialized: false };
                        }

                        const tokenInfo = parsed.value.data.parsed.info;
                        return {
                            token: tokenName,
                            mint: mint.toBase58(),
                            ata: ata.toBase58(),
                            balance: tokenInfo.tokenAmount.uiAmountString ?? "0",
                            decimals: tokenInfo.tokenAmount.decimals,
                            initialized: true,
                        };
                    }),
                );

                return { name, authority: pda.toBase58(), vaults };
            }),
        );

        printVaultList(groups, opts.json);
    });
}
