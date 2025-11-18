import { ScriptHelper } from "../utils/script-helper";

async function createCloseStateTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating close state transaction...");

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        const ix = await helper.buildCloseStateIx();

        const tx = await helper.prepareTransaction(ix);

        console.log("\n⚠️  CRITICAL WARNING: This will CLOSE the program state account!");
        console.log("This is a DESTRUCTIVE operation that will:");
        console.log("  - Permanently delete the program's main state account");
        console.log("  - Make the program effectively non-functional");
        console.log("  - Return the rent to the boss");
        console.log("\n⚠️  THIS CANNOT BE UNDONE!");

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