import { PublicKey } from "@solana/web3.js";
import { config, printConfigSummary, ScriptHelper } from "../utils/script-helper";

// Configuration - UPDATE THIS
// The public key of the new ONYC mint to set in program state
const NEW_ONYC_MINT = new PublicKey("HQmHPQLhuXTj8dbsLUoFsJeCZWBkK75Zwczxork8Byzh");

async function createSetOnycMintTransaction() {
    printConfigSummary(config);

    const helper = await ScriptHelper.create();

    console.log("Creating set ONYC mint transaction...");
    console.log("\n=== ONYC Mint Configuration ===");
    console.log("New ONYC Mint:", NEW_ONYC_MINT.toBase58());

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        const state = await helper.getState();

        console.log("\n=== Current ONYC Mint ===");
        const currentOnycMint = state.onycMint;
        console.log("Current:", currentOnycMint.toBase58());

        // Check if already set to this value
        if (currentOnycMint.equals(NEW_ONYC_MINT)) {
            console.log("\n⚠️  ONYC mint is already set to this value - no change needed");
            return;
        }

        console.log("\nBuilding transaction...");

        const ix = await helper.buildSetOnycMintIx({
            onycMint: NEW_ONYC_MINT,
            boss
        });

        const tx = await helper.prepareTransaction({ ix, payer: boss });

        console.log("\n=== Transaction Effects ===");
        console.log("This transaction will:");
        console.log(`  1. Update the program state's onyc_mint from ${currentOnycMint.toBase58()}`);
        console.log(`     to: ${NEW_ONYC_MINT.toBase58()}`);
        console.log("  2. Emit ONycMintUpdatedEvent");

        return helper.printTransaction(tx, "Set ONYC Mint Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createSetOnycMintTransaction();
    } catch (error) {
        console.error("Failed to create set ONYC mint transaction:", error);
    }
}

await main();
