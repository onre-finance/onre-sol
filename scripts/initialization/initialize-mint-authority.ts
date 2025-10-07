import { PublicKey } from "@solana/web3.js";
import { ScriptHelper, BOSS } from "../utils/script-helper";

async function createInitializeMintAuthorityTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating initialize mint authority transaction...");
    console.log("Boss:", BOSS.toBase58());

    try {
        const tx = await helper.buildInitializeMintAuthorityTransaction({
            boss: BOSS
        });

        return helper.printTransaction(tx, "Initialize Mint Authority Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createInitializeMintAuthorityTransaction();
    } catch (error) {
        console.error("Failed to create initialize mint authority transaction:", error);
    }
}

await main();
