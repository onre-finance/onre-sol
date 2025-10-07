import { PublicKey, Transaction } from "@solana/web3.js";
import { ScriptHelper, BOSS } from "../utils/script-helper";

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
        // Create all three transactions
        const migrateTx = await helper.buildMigrateStateTransaction({ boss: BOSS });
        const initMintTx = await helper.buildInitializeMintAuthorityTransaction({ boss: BOSS });
        const initVaultTx = await helper.buildInitializeVaultAuthorityTransaction({ boss: BOSS });

        console.log("\n=== Transaction Sequence ===");
        console.log("\n1. Migrate State Transaction:");
        const migrateTxBase58 = helper.serializeTransaction(migrateTx);
        console.log(migrateTxBase58);

        console.log("\n2. Initialize Mint Authority Transaction:");
        const initMintTxBase58 = helper.serializeTransaction(initMintTx);
        console.log(initMintTxBase58);

        console.log("\n3. Initialize Vault Authority Transaction:");
        const initVaultTxBase58 = helper.serializeTransaction(initVaultTx);
        console.log(initVaultTxBase58);

        console.log("\n=== Instructions ===");
        console.log("Execute these transactions IN ORDER:");
        console.log("1. First execute migrate state");
        console.log("2. Then execute initialize mint authority");
        console.log("3. Finally execute initialize vault authority");

        return {
            migrateState: migrateTxBase58,
            initializeMintAuthority: initMintTxBase58,
            initializeVaultAuthority: initVaultTxBase58
        };
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
