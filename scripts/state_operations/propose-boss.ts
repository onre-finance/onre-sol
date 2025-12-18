import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// New boss to propose - UPDATE THIS
const NEW_BOSS = new PublicKey("7rzEKejyAXJXMkGfRhMV9Vg1k7tFznBBEFu3sfLNz8LC");

async function createProposeBossTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating propose boss transaction...");
    console.log("New boss:", NEW_BOSS.toBase58());

    const currentBoss = await helper.getBoss();
    console.log("Current boss:", currentBoss.toBase58());

    try {
        const ix = await helper.buildProposeBossIx({
            boss: currentBoss,
            newBoss: NEW_BOSS
        });

        const tx = await helper.prepareTransaction({ ix, payer: currentBoss });

        console.log("\nThis is STEP 1 of the two-step ownership transfer:");
        console.log("  1. Current boss proposes a new boss (this transaction)");
        console.log("  2. Proposed boss accepts the transfer");
        console.log("\nAfter this transaction, the new boss must call accept-boss to complete the transfer.");

        return helper.printTransaction(tx, "Propose Boss Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createProposeBossTransaction();
    } catch (error) {
        console.error("Failed to create propose boss transaction:", error);
    }
}

await main();
