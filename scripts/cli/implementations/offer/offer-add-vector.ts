import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { addVectorParams } from "../../params";

/**
 * Execute offer add-vector command
 */
export async function executeOfferAddVector(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, addVectorParams, async (context) => {
        const { params } = context;

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                const boss = await helper.getBoss();
                return helper.buildAddOfferVectorIx({
                    tokenInMint: params.tokenIn,
                    tokenOutMint: params.tokenOut,
                    baseTime: params.baseTime,
                    basePrice: params.basePrice,
                    apr: params.apr,
                    priceFixDuration: params.duration,
                    boss,
                });
            },
            title: "Add Offer Vector Transaction",
            description: `Adds pricing vector to ${params.tokenIn.toBase58().slice(0, 8)}... → ${params.tokenOut.toBase58().slice(0, 8)}... offer`,
            showParamSummary: {
                title: "Adding price vector:",
                params: {
                    tokenIn: params.tokenIn,
                    tokenOut: params.tokenOut,
                    baseTime: new Date(params.baseTime * 1000).toISOString(),
                    basePrice: params.basePrice,
                    apr: `${params.apr / 10000}% (${params.apr})`,
                    duration: `${params.duration}s`,
                },
            },
        });
    });
}
