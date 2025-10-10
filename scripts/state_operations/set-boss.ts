import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// New boss to set - UPDATE THIS
const NEW_BOSS = new PublicKey("REPLACE_WITH_NEW_BOSS_PUBKEY");

async function createSetBossTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating set boss transaction...");
    console.log("New boss:", NEW_BOSS.toBase58());

    const currentBoss = await helper.getBoss();
    console.log("Current boss:", currentBoss.toBase58());

    try {
        const ix = await helper.buildSetBossIx({
            newBoss: NEW_BOSS
        });

        const tx = await helper.prepareTransaction(ix);

        return helper.printTransaction(tx, "Set Boss Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createSetBossTransaction();
    } catch (error) {
        console.error("Failed to create set boss transaction:", error);
    }
}

await main();
