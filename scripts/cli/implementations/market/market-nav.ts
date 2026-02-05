import { PublicKey } from "@solana/web3.js";
import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { tokenPairParams } from "../../params";
import { printNav } from "../../utils/display";

/**
 * Execute market nav command
 */
export async function executeMarketNav(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, tokenPairParams, async (context) => {
        const { helper, params } = context;

        // Call the view method
        const nav = await helper.program.methods
            .getNav()
            .accounts({
                tokenInMint: new PublicKey(params.tokenIn),
                tokenOutMint: new PublicKey(params.tokenOut),
            })
            .view();

        console.log("Raw nav:", nav.toNumber());
        printNav(nav.toNumber(), opts.json);
    });
}
