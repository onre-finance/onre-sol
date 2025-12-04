import { PublicKey } from "@solana/web3.js";
import { ScriptHelper, USDC_MINT, ONYC_MINT, USDC_TEST_MAINNET, ONYC_TEST_MAINNET, USDG_TEST_MAINNET, USDG_MINT } from "../utils/script-helper";
import { getMint, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Token addresses
const TOKEN_IN_MINT = USDC_MINT;
const TOKEN_OUT_MINT = ONYC_MINT;
const TOKEN_IN_PROGRAM = TOKEN_PROGRAM_ID;
const TOKEN_OUT_PROGRAM = TOKEN_PROGRAM_ID;

// Helper function to format timestamp to human readable date
function formatTimestamp(timestamp: number): string {
    if (timestamp === 0) return "Not set";
    return new Date(timestamp * 1000).toISOString();
}

async function fetchOffer() {
    const helper = await ScriptHelper.create();

    console.log("Fetching offer details...");
    console.log("Token In (USDC):", TOKEN_IN_MINT.toBase58());
    console.log("Token Out (ONyc):", TOKEN_OUT_MINT.toBase58());

    try {
        const offer = await helper.getOffer(TOKEN_IN_MINT, TOKEN_OUT_MINT);
        if (!offer) {
            console.log(`Offer for ${TOKEN_IN_MINT.toBase58()} -> ${TOKEN_OUT_MINT.toBase58()} not found.`);
            return;
        }

        console.log("\n📋 OFFER DETAILS");
        console.log("================");
        console.log(`Token In:  ${offer.tokenInMint.toBase58()}`);
        console.log(`Token Out: ${offer.tokenOutMint.toBase58()}`);
        console.log(`Fee: ${(offer.feeBasisPoints / 100).toFixed(2)}%`);
        console.log(`Needs Approval: ${offer.needsApproval != 0}`);
        console.log(`Allow Permissionless: ${offer.allowPermissionless != 0}`);

        // Fetch token mint info for better display
        try {
            const tokenInMint = await getMint(helper.connection, offer.tokenInMint, 'confirmed', TOKEN_IN_PROGRAM);
            const tokenOutMint = await getMint(helper.connection, offer.tokenOutMint, 'confirmed', TOKEN_OUT_PROGRAM);
            console.log(`Token In Decimals: ${tokenInMint.decimals}`);
            console.log(`Token Out Decimals: ${tokenOutMint.decimals}`);
        } catch (error) {
            console.log("Could not fetch mint info:", error);
        }

        // Show vectors
        const activeVectors = offer.vectors.filter(v => v.startTime.toNumber() > 0);
        console.log(`\nVectors: ${activeVectors.length} configured`);

        if (activeVectors.length > 0) {
            console.log("\n🔢 VECTOR DETAILS");
            console.log("=================");

            for (let i = 0; i < activeVectors.length; i++) {
                const vector = activeVectors[i];
                console.log(`\nVector #${i}:`);
                console.log(`  Start Time: ${formatTimestamp(vector.startTime.toNumber())} (${vector.startTime})`);
                console.log(`  Base Time:  ${formatTimestamp(vector.baseTime.toNumber())} (${vector.baseTime})`);
                console.log(`  Base Price: ${vector.basePrice.toString()}`);
                console.log(`  APR: ${(vector.apr.toNumber() / 1_000_000).toFixed(4)}%`);
                console.log(`  Price Fix Duration: ${vector.priceFixDuration.toNumber()}s`);

                // Check if vector is currently active
                const now = Math.floor(Date.now() / 1000);
                const isActive = now >= vector.startTime.toNumber();
                console.log(`  Status: ${isActive ? "🟢 ACTIVE" : "🔴 PENDING"}`);
            }
        } else {
            console.log("\n⚠️  No vectors configured for this offer");
        }

    } catch (error) {
        console.error("Error fetching offer:", error);
    }
}

async function main() {
    try {
        await fetchOffer();
    } catch (error) {
        console.error("Failed to fetch offer:", error);
    }
}

await main();
