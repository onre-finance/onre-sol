import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { tokenPairParams } from "../../params";
import { printTvl } from "../../utils/display";
import { getTokenProgramId } from "../../utils/token-utils";

/**
 * Execute market tvl command
 */
export async function executeMarketTvl(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, tokenPairParams, async (context) => {
        const { helper, params } = context;

        // Determine the correct token program for tokenOut
        const tokenOutProgram = getTokenProgramId(params.tokenOut);

        // Get the offer vault token account
        const offerVaultTokenOut = getAssociatedTokenAddressSync(
            params.tokenOut,
            helper.pdas.offerVaultAuthorityPda,
            true,
            tokenOutProgram
        );

        // Call the view method
        const tvl = await helper.program.methods
            .getTvl()
            .accounts({
                tokenInMint: new PublicKey(params.tokenIn),
                tokenOutMint: new PublicKey(params.tokenOut),
                vaultTokenOutAccount: offerVaultTokenOut,
                tokenOutProgram
            })
            .view();

        // Use toString() to avoid BN serialization issues with large numbers
        const tvlValue = tvl.toString();
        printTvl(tvlValue, opts.json);
    });
}
