import chalk from "chalk";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { config } from "../../utils/script-helper";
import type { GlobalOptions } from "../prompts";
import { executeCommand } from "../helpers";
import { printCirculatingSupply } from "../utils/display";

/**
 * Execute market supply command
 */
export async function executeMarketSupply(
    opts: GlobalOptions
): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        const { helper } = context;

        // Get the offer vault token account
        const onycVaultAccount = getAssociatedTokenAddressSync(
            config.mints.onyc,
            helper.pdas.offerVaultAuthorityPda,
            true
        );

        // Call the view method
        const supply = await helper.program.methods
            .getCirculatingSupply()
            .accounts({
                onycVaultAccount,
                tokenProgram: TOKEN_PROGRAM_ID
            })
            .view();

        printCirculatingSupply(supply.toNumber(), opts.json);
    });
}
