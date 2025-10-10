import { ScriptHelper } from "../utils/script-helper";

async function createInitializeVaultTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating initialize vault authority transaction...");

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());
    console.log("State PDA:", helper.statePda.toBase58());
    console.log("Offer Vault Authority PDA:", helper.pdas.offerVaultAuthorityPda.toBase58());

    try {
        const ix = await helper.buildInitializeVaultAuthorityIx();
        const tx = await helper.prepareTransaction(ix);

        console.log("\nThis transaction will initialize:");
        console.log("- Offer vault authority PDA for managing program-owned token accounts");
        console.log("- Required for offer vault deposit/withdraw operations");

        return helper.printTransaction(tx, "Initialize Vault Authority Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createInitializeVaultTransaction();
    } catch (error) {
        console.error("Failed to create initialize vault transaction:", error);
    }
}

await main();