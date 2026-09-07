import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { listRequestsParams } from "../../params";
import { printRedemptionRequestsList } from "../../utils/display";

/**
 * Execute redemption list-requests command
 */
export async function executeRedemptionListRequests(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, listRequestsParams, async (context) => {
        const { helper, params } = context;

        // Fetch all redemption requests for this token pair
        const redemptionOfferPda = helper.getRedemptionOfferPda(params.tokenIn, params.tokenOut);

        // Build filters - always filter by offer
        const filters: any[] = [
            {
                memcmp: {
                    offset: 8, // After discriminator - filter by offer PDA
                    bytes: redemptionOfferPda.toBase58(),
                },
            },
        ];
        if (params.redeemer) {
            filters.push({
                memcmp: {
                    offset: 76, // discriminator + offer + fixed 32-byte string encoding
                    bytes: params.redeemer.toBase58(),
                },
            });
        }

        const requests = await helper.program.account.redemptionRequest.all(filters);

        // Transform and sort by frontend-generated request ID.
        const formattedRequests = requests
            .map((r) => ({
                id: r.account.requestId as string,
                request: r.account,
            }))
            .sort((a, b) => a.id.localeCompare(b.id));

        printRedemptionRequestsList(formattedRequests, opts.json);
    });
}
