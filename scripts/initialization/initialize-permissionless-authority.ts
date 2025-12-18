import { config, ScriptHelper } from "../utils/script-helper";

const PERMISSIONLESS_NAME = "permissionless-1";

async function createInitializePermissionlessAuthorityTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating initialize permissionless authority transaction...");
    console.log("Boss:", config.boss.toBase58());
    console.log("Name:", PERMISSIONLESS_NAME);

    try {
        const ix = await helper.buildInitializePermissionlessAuthorityIx({
            name: PERMISSIONLESS_NAME,
            boss: config.boss
        });

        const tx = await helper.prepareTransaction({ ix, payer: config.boss });

        return helper.printTransaction(tx, "Initialize Permissionless Authority Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createInitializePermissionlessAuthorityTransaction();
    } catch (error) {
        console.error("Failed to create initialize permissionless authority transaction:", error);
    }
}

await main();
