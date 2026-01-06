import { TransactionInstruction } from "@solana/web3.js";
import { config, printConfigSummary, ScriptHelper } from "../utils/script-helper";
import { TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";

const TOKEN_IN_MINT = config.mints.onyc;
const TOKEN_OUT_MINT = config.mints.usdc;
const TOKEN_IN_PROGRAM = TOKEN_PROGRAM_ID;
const TOKEN_OUT_PROGRAM = TOKEN_OUT_MINT == config.mints.usdg ? TOKEN_2022_PROGRAM_ID : TOKEN_PROGRAM_ID;

// Offer configuration
const FEE_BASIS_POINTS = 0; // 0% fee

async function createRedemptionOfferTransaction() {
    printConfigSummary(config);
    const helper = await ScriptHelper.create();

    console.log("Creating redemption offer transaction...");
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
            boss
        });
        instructions.push(makeRedemptionOfferIx);
        console.log("\n1. Added make redemption offer instruction");

        const tx = await helper.prepareTransactionMultipleIxs({ ixs: instructions, payer: boss });

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