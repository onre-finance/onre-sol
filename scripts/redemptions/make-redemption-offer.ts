import { TransactionInstruction } from "@solana/web3.js";
import { ScriptHelper, USDC_MINT, USDG_MINT, ONYC_MINT } from "../utils/script-helper";
import { TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";

const TOKEN_IN_MINT = ONYC_MINT;
const TOKEN_OUT_MINT = USDC_MINT;
const TOKEN_IN_PROGRAM = TOKEN_PROGRAM_ID;
const TOKEN_OUT_PROGRAM = TOKEN_PROGRAM_ID;

// Offer configuration
const FEE_BASIS_POINTS = 0; // 0% fee

async function createRedemptionOfferTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creatigng redemptionoffer transaction...");
    const boss = await helper.getBoss();
    console.log("\nBoss:", boss.toBase58());

    try {
        const instructions: TransactionInstruction[] = [];

        // 1. Make Offer instruction
        const makeRedemptionOfferIx = await helper.buildMakeRedemptionOfferIx({
            tokenInMint: TOKEN_IN_MINT,
            tokenInProgram: TOKEN_IN_PROGRAM,
            tokenOutMint: TOKEN_OUT_MINT,
            tokenOutProgram: TOKEN_OUT_PROGRAM,
            feeBasisPoints: FEE_BASIS_POINTS,
        });
        instructions.push(makeRedemptionOfferIx);
        console.log("\n1. Added make redemption offer instruction");

        const tx = await helper.prepareTransactionMultipleIxs(instructions);

        return helper.printTransaction(tx, "Make Redemption Offer Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createRedemptionOfferTransaction();
    } catch (error) {
        console.error("Failed to create redemptionoffer transaction:", error);
    }
}

await main();
