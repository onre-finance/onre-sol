import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { tokenPairParams } from "../../params";
import { printRedemptionOffer } from "../../utils/display";

/**
 * Execute redemption fetch-offer command
 */
export async function executeRedemptionFetchOffer(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, tokenPairParams, async (context) => {
        const { helper, params } = context;

        // Fetch the redemption offer account
        const offerAddress = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);
        console.log(`Redemption Offer PDA: ${offerAddress}`);
        const offer = await helper.program.account.redemptionOffer.fetch(offerAddress);

        printRedemptionOffer(offer, params.tokenIn.toBase58(), params.tokenOut.toBase58(), opts.json);
    });
}
