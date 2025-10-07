import { ScriptHelper } from "../utils/script-helper";

async function createKillSwitchDisableTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating kill switch disable transaction...");

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        const tx = await helper.buildSetKillSwitchTransaction({
            enable: false
        });

        console.log("\n✅ This will DISABLE the kill switch!");
        console.log("This will restore normal program operations.");

        return helper.printTransaction(tx, "Kill Switch Disable Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createKillSwitchDisableTransaction();
    } catch (error) {
        console.error("Failed to create kill switch disable transaction:", error);
    }
}

await main();