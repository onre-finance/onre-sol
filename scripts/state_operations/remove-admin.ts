import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// Admin to remove - UPDATE THIS
const ADMIN_TO_REMOVE = new PublicKey("REPLACE_WITH_ADMIN_PUBKEY");

async function createRemoveAdminTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating remove admin transaction...");
    console.log("Admin to remove:", ADMIN_TO_REMOVE.toBase58());

    const boss = await helper.getBoss();
    console.log("Current boss:", boss.toBase58());

    try {
        const ix = await helper.buildRemoveAdminIx({
            admin: ADMIN_TO_REMOVE,
            boss
        });

        const tx = await helper.prepareTransaction({ ix, payer: boss });

        return helper.printTransaction(tx, "Remove Admin Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createRemoveAdminTransaction();
    } catch (error) {
        console.error("Failed to create remove admin transaction:", error);
    }
}

await main();