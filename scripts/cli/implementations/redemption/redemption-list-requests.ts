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

        // Conditionally add redeemer filter if provided
        if (params.redeemer) {
            filters.push({
                memcmp: {
                    offset: 8 + 32 + 8, // After discriminator + offer + request_id - filter by redeemer
                    bytes: params.redeemer.toBase58(),
                },
            });
        }

        const requests = await helper.program.account.redemptionRequest.all(filters);

        // Transform and sort the data by ID
        const formattedRequests = requests
            .map((r) => ({
                id: r.account.requestId.toNumber(),
                request: r.account,
            }))
            .sort((a, b) => a.id - b.id);

        printRedemptionRequestsList(formattedRequests, opts.json);
    });
}
