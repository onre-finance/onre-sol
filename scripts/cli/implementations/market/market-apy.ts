import { PublicKey } from "@solana/web3.js";
import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { tokenPairParams } from "../../params";
import { printApy } from "../../utils/display";

/**
 * Execute market apy command
 */
export async function executeMarketApy(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, tokenPairParams, async (context) => {
        const { helper, params } = context;

        // Call the view method
        const apy = await helper.program.methods
            .getApy()
            .accounts({
                tokenInMint: new PublicKey(params.tokenIn),
                tokenOutMint: new PublicKey(params.tokenOut),
            })
            .view();

        printApy(apy.toNumber(), opts.json);
    });
}
