import { PublicKey } from "@solana/web3.js";
import { ScriptHelper, USDC_MINT, ONYC_MINT, USDC_TEST_MAINNET, ONYC_TEST_MAINNET, USDG_MINT } from "../utils/script-helper";

type VectorInput = {
    baseTime: number;
    basePrice: number;
    apr: number;
    tokenIn: PublicKey;
    tokenOut: PublicKey;
};

async function createAddOfferVectorTransaction(vector: VectorInput) {

    const BASE_TIME = vector.baseTime / 1000;
    const BASE_PRICE = Math.round(vector.basePrice * 1_000_000_000);
    const APR = Math.round(vector.apr * 10_000);
    const PRICE_FIX_DURATION = 60 * 60 * 24; // 1 day in seconds

    const TOKEN_IN_MINT = vector.tokenIn;
    const TOKEN_OUT_MINT = vector.tokenOut;


    const helper = await ScriptHelper.create();

    console.log("Creating add offer vector transaction...");
    console.log("Token In (USDC):", TOKEN_IN_MINT.toBase58());
    console.log("Token Out (ONe):", TOKEN_OUT_MINT.toBase58());
    console.log("Base Time:", new Date(BASE_TIME * 1000).toISOString());
    console.log("Base Price:", BASE_PRICE / 1_000_000_000);
    console.log("APR:", APR / 10_000, "%");
    console.log("Price Fix Duration:", PRICE_FIX_DURATION, "seconds");

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        // Check if offer exists
        const offer = await helper.getOffer(TOKEN_IN_MINT, TOKEN_OUT_MINT);
        if (!offer) {
            throw new Error(`Offer for ${TOKEN_IN_MINT.toBase58()} -> ${TOKEN_OUT_MINT.toBase58()} not found. Create the offer first using make-offer script.`);
        }

        console.log("Found offer:", {
            tokenIn: offer.tokenInMint.toBase58(),
            tokenOut: offer.tokenOutMint.toBase58(),
            feeBasisPoints: offer.feeBasisPoints,
            needsApproval: offer.needsApproval != 0,
            allowPermissionless: offer.allowPermissionless != 0,
            vectors: offer.vectors.filter(v => v.startTime.toNumber() > 0).length
        });

        const ix = await helper.buildAddOfferVectorIx({
            tokenInMint: TOKEN_IN_MINT,
            tokenOutMint: TOKEN_OUT_MINT,
            baseTime: BASE_TIME,
            basePrice: BASE_PRICE,
            apr: APR,
            priceFixDuration: PRICE_FIX_DURATION,
            boss: boss,
        });

        const tx = await helper.prepareTransaction(ix);

        return helper.printTransaction(tx, "Add Offer Vector Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createAddOfferVectorTransaction({
            baseTime: Date.UTC(2025, 11, 4, 0, 0, 0),
            basePrice: 1.0541519,
            apr: 12.665,
            tokenIn: USDG_MINT,
            tokenOut: ONYC_MINT,
        });

    } catch (error) {
        console.error("Failed to create add offer vector transaction:", error);
    }
}

await main();
