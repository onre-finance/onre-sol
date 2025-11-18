import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Token addresses - UPDATE THESE
const TOKEN_IN_MINT = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"); // USDC
// const TOKEN_IN_MINT = new PublicKey("2u1tszSeqZ3qBWF3uNGPFc8TzMk2tdiwknnRMWGWjGWH"); // USDG
const TOKEN_OUT_MINT = new PublicKey("5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5"); // ONyc
const TOKEN_IN_PROGRAM = TOKEN_PROGRAM_ID; // Token program for TOKEN_IN_MINT (use TOKEN_PROGRAM_ID or TOKEN_2022_PROGRAM_ID)

// Offer configuration
const FEE_BASIS_POINTS = 0; // 0% fee
const NEEDS_APPROVAL = false; // No approval required
const ALLOW_PERMISSIONLESS = true; // Enable permissionless

// Vector configuration
const BASE_TIME = Math.floor(new Date(Date.UTC(2025, 4, 27, 0, 0, 0)).getTime() / 1000); // May 27, 2025
const BASE_PRICE = 1_000_000_000; // 1.0 (scaled by 1,000,000,000) all prices are scaled by 9 decimals
const APR = 36_500; // 3.65% APR
const PRICE_FIX_DURATION = 60 * 60 * 24; // 1 day

async function createMakeOfferWithVectorTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating make offer with vector transaction...");
    console.log("\nOffer Configuration:");
    console.log("  Token In (USDC):", TOKEN_IN_MINT.toBase58());
    console.log("  Token Out (ONe):", TOKEN_OUT_MINT.toBase58());
    console.log("  Token In Program:", TOKEN_IN_PROGRAM.toBase58());
    console.log("  Fee Basis Points:", FEE_BASIS_POINTS);
    console.log("  Needs Approval:", NEEDS_APPROVAL);
    console.log("  Allow Permissionless:", ALLOW_PERMISSIONLESS);

    console.log("\nVector Configuration:");
    console.log("  Base Time:", new Date(BASE_TIME * 1000).toISOString());
    console.log("  Base Price:", BASE_PRICE);
    console.log("  APR:", APR / 10_000, "%");
    console.log("  Price Fix Duration:", PRICE_FIX_DURATION, "seconds");

    const boss = await helper.getBoss();
    console.log("\nBoss:", boss.toBase58());

    try {
        const instructions: TransactionInstruction[] = [];

        // If permissionless is enabled, create the intermediary token accounts
        if (ALLOW_PERMISSIONLESS) {
            const permissionlessIxs = await helper.buildCreatePermissionlessTokenAccountsIxs({
                tokenInMint: TOKEN_IN_MINT,
                tokenOutMint: TOKEN_OUT_MINT
            });
            instructions.push(...permissionlessIxs);

            if (permissionlessIxs.length > 0) {
                console.log(`\nAdded ${permissionlessIxs.length} instruction(s) to create permissionless token accounts`);
            } else {
                console.log("\nPermissionless token accounts already exist");
            }
        }

        // 1. Make Offer instruction
        const makeOfferIx = await helper.buildMakeOfferIx({
            tokenInMint: TOKEN_IN_MINT,
            tokenOutMint: TOKEN_OUT_MINT,
            feeBasisPoints: FEE_BASIS_POINTS,
            needsApproval: NEEDS_APPROVAL,
            allowPermissionless: ALLOW_PERMISSIONLESS,
            tokenInProgram: TOKEN_IN_PROGRAM
        });
        instructions.push(makeOfferIx);
        console.log("\n1. Added make offer instruction");

        // 2. Add Offer Vector instruction
        const addVectorIx = await helper.buildAddOfferVectorIx({
            tokenInMint: TOKEN_IN_MINT,
            tokenOutMint: TOKEN_OUT_MINT,
            baseTime: BASE_TIME,
            basePrice: BASE_PRICE,
            apr: APR,
            priceFixDuration: PRICE_FIX_DURATION
        });
        instructions.push(addVectorIx);
        console.log("2. Added offer vector instruction");

        const tx = await helper.prepareTransactionMultipleIxs(instructions);

        console.log("\nThis transaction will:");
        console.log("  1. Create the offer (if permissionless: also create intermediary token accounts)");
        console.log("  2. Add the first vector to the offer");
        console.log("\nBoth operations will be executed atomically in a single transaction.");

        return helper.printTransaction(tx, "Make Offer with Vector Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createMakeOfferWithVectorTransaction();
    } catch (error) {
        console.error("Failed to create make offer with vector transaction:", error);
    }
}

await main();
