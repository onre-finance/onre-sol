import { ScriptHelper } from "../utils/script-helper";

async function createKillSwitchEnableTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating kill switch enable transaction...");

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        const ix = await helper.buildSetKillSwitchIx({
            enable: true
        });

        const tx = await helper.prepareTransaction(ix);

        console.log("\n⚠️  WARNING: This will ENABLE the kill switch!");
        console.log("This will disable critical program operations.");

        return helper.printTransaction(tx, "Kill Switch Enable Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createKillSwitchEnableTransaction();
    } catch (error) {
        console.error("Failed to create kill switch enable transaction:", error);
    }
}

await main();