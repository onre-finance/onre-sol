import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

async function getBoss() {
    const helper = await ScriptHelper.create();

    console.log("Fetching current boss...");

    try {
        const boss = await helper.getBoss();
        const state = await helper.getState();

        console.log("Current boss:", boss.toBase58());
        console.log("Kill switch enabled:", state.isKilled);
        console.log("Admin count:", state.admins.filter(admin => !admin.equals(new PublicKey("11111111111111111111111111111111"))).length);

        return boss;
    } catch (error) {
        console.error("Error fetching boss:", error);
        throw error;
    }
}

async function main() {
    try {
        await getBoss();
    } catch (error) {
        console.error("Failed to get boss:", error);
    }
}

await main();