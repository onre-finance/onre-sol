import { ScriptHelper } from "../utils/script-helper";

async function createMigrateStateTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating migrate state transaction...");

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        const tx = await helper.buildMigrateStateTransaction();

        console.log("\nThis transaction will:");
        console.log("- Migrate the State account to include new fields");
        console.log("- Reallocate account to new size");
        console.log("- Initialize kill switch to disabled by default");

        return helper.printTransaction(tx, "Migrate State Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createMigrateStateTransaction();
    } catch (error) {
        console.error("Failed to create migrate state transaction:", error);
    }
}

await main();