import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { tokenPairParams } from "../../params";
import { printOffer } from "../../utils/display";

/**
 * Execute offer fetch command
 */
export async function executeOfferFetch(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, tokenPairParams, async (context) => {
        const { helper, params } = context;

        // Fetch the offer account
        const offer = await helper.getOffer(params.tokenIn, params.tokenOut);

        // Convert PublicKeys to base58 strings for display
        printOffer(offer, params.tokenIn.toBase58(), params.tokenOut.toBase58(), opts.json);
    });
}
