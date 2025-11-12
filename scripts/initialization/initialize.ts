import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// Use DEV Squad for initialization
const BOSS = new PublicKey("EVdiVScB7LX1P3bn7ZLmLJTBrSSgRXPqRU3bVxrEpRb5"); // DEV Squad

async function createInitializeTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating initialize transaction...");
    console.log("Boss (DEV Squad):", BOSS.toBase58());

    try {
        const ix = await helper.buildInitializeIx();

        const tx = await helper.prepareTransaction(ix, BOSS);

        return helper.printTransaction(tx, "Initialize Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createInitializeTransaction();
    } catch (error) {
        console.error("Failed to create initialize transaction:", error);
    }
}

await main();