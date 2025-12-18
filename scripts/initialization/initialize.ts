import { config, printConfigSummary, ScriptHelper } from "../utils/script-helper";

async function createInitializeTransaction() {
    printConfigSummary(config);

    const helper = await ScriptHelper.create();

    console.log("Creating initialize transaction...");
    console.log("Boss:", config.boss.toBase58());

    try {
        const ix = await helper.buildInitializeIx({ boss: config.boss });

        const tx = await helper.prepareTransaction({ ix, payer: config.boss });

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
