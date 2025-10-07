import { BOSS, ScriptHelper } from "../utils/script-helper";

/**
 * Creates a sequence of instructions for program upgrade:
 * 1. Migrate state
 * 2. Initialize mint authority
 * 3. Initialize vault authority
 */
async function createUpgradeSequence() {
    const helper = await ScriptHelper.create();

    console.log("Creating program upgrade sequence...");
    console.log("Boss:", BOSS.toBase58());
    console.log("State PDA:", helper.statePda.toBase58());
    console.log("Mint Authority PDA:", helper.pdas.mintAuthorityPda.toBase58());
    console.log("Vault Authority PDA:", helper.pdas.offerVaultAuthorityPda.toBase58());

    try {
        // Create all three instructions
        const migrateIx = await helper.buildMigrateStateIx();
        const initMintIx = await helper.buildInitializeMintAuthorityIx();
        const initVaultIx = await helper.buildInitializeVaultAuthorityIx();

        const tx = await helper.prepareTransactionMultipleIxs([migrateIx, initMintIx, initVaultIx]);

        console.log("\n=== Instruction Sequence ===");
        console.log("\n1. Migrate State Instruction:");
        console.log("\n2. Initialize Mint Authority Instruction:");
        console.log("\n3. Initialize Vault Authority Instruction:");

        return helper.printTransaction(tx, "Upgrade Sequence Transaction");
    } catch (error) {
        console.error("Error creating upgrade sequence:", error);
        throw error;
    }
}

async function main() {
    try {
        await createUpgradeSequence();
    } catch (error) {
        console.error("Failed to create upgrade sequence:", error);
    }
}

await main();
