import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// Admin to add - UPDATE THIS
const NEW_ADMIN = new PublicKey("DHcViDjSYyPaeSE8QzoasqbDeZHDVXGoC1ReM9FtRFCq");

async function createAddAdminTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating add admin transaction...");
    console.log("New admin:", NEW_ADMIN.toBase58());

    const boss = await helper.getBoss();
    console.log("Current boss:", boss.toBase58());

    try {
        const ix = await helper.buildAddAdminIx({
            admin: NEW_ADMIN
        });

        const tx = await helper.prepareTransaction(ix);

        return helper.printTransaction(tx, "Add Admin Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createAddAdminTransaction();
    } catch (error) {
        console.error("Failed to create add admin transaction:", error);
    }
}

await main();
