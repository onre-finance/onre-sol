import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

async function createCloseStateTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating close state transaction...");
    console.log("WARNING: This will permanently close the program state account!");
    console.log("This operation cannot be undone and will disable the program.");

    try {
        // Try to fetch and display current state (may fail if state structure is outdated)
        try {
            const boss = new PublicKey("EVdiVScB7LX1P3bn7ZLmLJTBrSSgRXPqRU3bVxrEpRb5");
            console.log("\nBoss:", boss.toBase58());

            const state = await helper.getState();
            console.log("\nCurrent State:");
            console.log("  Boss:", state.boss.toBase58());
            console.log("  ONyc Mint:", state.onycMint.toBase58());
            console.log("  Kill Switch:", state.isKilled ? "ENABLED" : "disabled");
            console.log("  Max Supply:", state.maxSupply.toString());
            console.log("  Admins:", state.admins.filter(admin => admin.toBase58() !== PublicKey.default.toBase58()).map(a => a.toBase58()));
        } catch (stateError) {
            console.log("\nNote: Could not deserialize state account (may have outdated structure).");
            console.log("This is expected if the on-chain state doesn't match current IDL.");
            console.log("The close_state instruction will still work correctly.\n");
        }

        const ix = await helper.buildCloseStateIx();

        const tx = await helper.prepareTransaction(ix);

        return helper.printTransaction(tx, "Close State Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createCloseStateTransaction();
    } catch (error) {
        console.error("Failed to create close state transaction:", error);
    }
}

await main();
