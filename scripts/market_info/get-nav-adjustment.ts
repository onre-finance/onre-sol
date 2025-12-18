import { PublicKey } from "@solana/web3.js";
import { ScriptHelper, config } from "../utils/script-helper";

// Token addresses - automatically use the correct mints for the selected network
const TOKEN_IN_MINT = config.mints.usdc;
const TOKEN_OUT_MINT = config.mints.onyc;

async function getNAVAdjustment() {
    const helper = await ScriptHelper.create();

    console.log("Fetching NAV adjustment for offer...");
    console.log("Token In Mint:", TOKEN_IN_MINT.toBase58());
    console.log("Token Out Mint:", TOKEN_OUT_MINT.toBase58());

    try {
        const adjustment = await helper.program.methods
            .getNavAdjustment()
            .accounts({
                tokenInMint: TOKEN_IN_MINT,
                tokenOutMint: TOKEN_OUT_MINT
            })
            .view();

        const adjustmentNumber = adjustment.toNumber();

        console.log("\n=== NAV Adjustment Results ===");
        console.log(`Adjustment (raw): ${adjustmentNumber}`);
        console.log(`Adjustment (decimal): ${(adjustmentNumber / 1_000_000_000).toFixed(9)}`);
        console.log(`Direction: ${adjustmentNumber >= 0 ? "Increase" : "Decrease"}`);

        return adjustmentNumber;
    } catch (error) {
        console.error("Error fetching NAV adjustment:", error);
        throw error;
    }
}

async function main() {
    try {
        await getNAVAdjustment();
    } catch (error) {
        console.error("Failed to get NAV adjustment:", error);
    }
}

await main();
