import { PublicKey } from "@solana/web3.js";
import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { tokenPairParams } from "../../params";
import { printNavAdjustment } from "../../utils/display";

/**
 * Execute market nav-adjustment command
 */
export async function executeMarketNavAdjustment(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, tokenPairParams, async (context) => {
        const { helper, params } = context;

        // Call the view method
        const nav = await helper.program.methods
            .getNavAdjustment()
            .accounts({
                tokenInMint: new PublicKey(params.tokenIn),
                tokenOutMint: new PublicKey(params.tokenOut),
            })
            .view();

        console.log("Raw nav adjustment:", nav.toNumber());
        printNavAdjustment(nav.toNumber(), opts.json);
    });
}
