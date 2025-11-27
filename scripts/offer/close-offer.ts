import { ONYC_MINT, ScriptHelper, USDC_MINT,  } from "../utils/script-helper";

// Configure which mints to close
const TOKEN_IN_MINT = USDC_MINT;
const TOKEN_OUT_MINT = ONYC_MINT;

async function createCloseOfferTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating close offer transaction...");
    console.log("Token In (USDC):", TOKEN_IN_MINT.toBase58());
    console.log("Token Out (ONe):", TOKEN_OUT_MINT.toBase58());

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        // Check if offer exists
        const offer = await helper.getOffer(TOKEN_IN_MINT, TOKEN_OUT_MINT);
        if (!offer) {
            throw new Error(`Offer for ${TOKEN_IN_MINT.toBase58()} -> ${TOKEN_OUT_MINT.toBase58()} not found.`);
        }

        console.log("Found offer:", {
            tokenIn: offer.tokenInMint.toBase58(),
            tokenOut: offer.tokenOutMint.toBase58(),
            feeBasisPoints: offer.feeBasisPoints,
            needsApproval: offer.needsApproval,
            allowPermissionless: offer.allowPermissionless,
            vectors: offer.vectors.filter(v => v.startTime.toNumber() > 0).length
        });

        const ix = await helper.buildCloseOfferIx({
            tokenInMint: TOKEN_IN_MINT,
            tokenOutMint: TOKEN_OUT_MINT
        });

        const tx = await helper.prepareTransaction(ix);

        return helper.printTransaction(tx, "Close Offer Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createCloseOfferTransaction();
    } catch (error) {
        console.error("Failed to create close offer transaction:", error);
    }
}

await main();