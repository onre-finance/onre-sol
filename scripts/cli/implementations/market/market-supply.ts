import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { config } from "../../../utils/script-helper";
import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { printCirculatingSupply } from "../../utils/display";
import { getTokenProgramId } from "../../utils/token-utils";

/**
 * Execute market supply command
 */
export async function executeMarketSupply(opts: GlobalOptions): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        const { helper } = context;

        // Determine the correct token program for ONyc
        const tokenProgram = getTokenProgramId(config.mints.onyc);

        // Get the offer vault token account
        const onycVaultAccount = getAssociatedTokenAddressSync(config.mints.onyc, helper.pdas.offerVaultAuthorityPda, true, tokenProgram);

        // Call the view method
        const supply = await helper.program.methods
            .getCirculatingSupply()
            .accounts({
                onycVaultAccount,
                tokenProgram,
            })
            .view();

        // Use toString() to avoid BN serialization issues with large numbers
        const supplyValue = supply.toString();
        printCirculatingSupply(supplyValue, opts.json);
    });
}
