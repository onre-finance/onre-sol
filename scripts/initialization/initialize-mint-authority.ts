import { BOSS, ScriptHelper } from "../utils/script-helper";

async function createInitializeMintAuthorityTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating initialize mint authority transaction...");
    console.log("Boss:", BOSS.toBase58());

    try {
        const ix = await helper.buildInitializeMintAuthorityIx();
        const tx = await helper.prepareTransaction(ix);

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
