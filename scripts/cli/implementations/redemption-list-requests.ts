import type { GlobalOptions } from "../prompts";
import { executeCommand } from "../helpers";
import { tokenPairParams } from "../params/redemption";
import { printRedemptionRequestsList } from "../utils/display";

/**
 * Execute redemption list-requests command
 */
export async function executeRedemptionListRequests(
    opts: GlobalOptions & Record<string, any>
): Promise<void> {
    await executeCommand(opts, tokenPairParams, async (context) => {
        const { helper, params } = context;

        // Fetch all redemption requests for this token pair
        const boss = await helper.getBoss();
        const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);

        const requests = await helper.program.account.redemptionRequest.all([
            {
                memcmp: {
                    offset: 8, // After discriminator - filter by offer PDA
                    bytes: redemptionOfferPda.toBase58()
                }
            },
            {
                memcmp: {
                    offset: 8 + 32 + 8, // After discriminator + offer + request_id - filter by redeemer
                    bytes: boss.toBase58()
                }
            }
        ]);

        // Transform the data to match the expected format
        const formattedRequests = requests.map(r => ({
            id: r.account.requestId.toNumber(),
            request: r.account
        }));

        printRedemptionRequestsList(formattedRequests, opts.json);
    });
}
