import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// Same initializer as deployer
const BOSS = new PublicKey("onREP1E8Yk7p83fRFeApZSTN1vCzdfZKzS46dWErsre");

async function createInitializeTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating initialize transaction...");
    console.log("Boss:", BOSS.toBase58());

    try {
        const ix = await helper.buildInitializeIx({ boss: BOSS });

        const tx = await helper.prepareTransaction({ ix, payer: BOSS });

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
