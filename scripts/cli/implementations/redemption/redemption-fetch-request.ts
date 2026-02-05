import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { requestParams } from "../../params";
import { printRedemptionRequest } from "../../utils/display";

/**
 * Execute redemption fetch-request command
 */
export async function executeRedemptionFetchRequest(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, requestParams, async (context) => {
        const { helper, params } = context;

        // Fetch the redemption request account
        const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);
        const requestAddress = helper.getRedemptionRequestPda(redemptionOfferPda, params.requestId);
        const request = await helper.program.account.redemptionRequest.fetch(requestAddress);

        printRedemptionRequest(request, params.requestId, opts.json);
    });
}
