import chalk from "chalk";
import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import type { GlobalOptions } from "../prompts";
import { executeCommand } from "../helpers";
import { tokenPairParams } from "../params";
import { printTvl } from "../utils/display";

/**
 * Execute market tvl command
 */
export async function executeMarketTvl(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, tokenPairParams, async (context) => {
        const { helper, params } = context;

        // Get the offer vault token account
        const offerVaultTokenOut = getAssociatedTokenAddressSync(
            params.tokenOut,
            helper.pdas.offerVaultAuthorityPda,
            true
        );

        // Call the view method
        const tvl = await helper.program.methods
            .getTvl()
            .accounts({
                tokenInMint: new PublicKey(params.tokenIn),
                tokenOutMint: new PublicKey(params.tokenOut),
                vaultTokenOutAccount: offerVaultTokenOut,
                tokenOutProgram: TOKEN_PROGRAM_ID
            })
            .view();

        printTvl(tvl.toNumber(), opts.json);
    });
}
