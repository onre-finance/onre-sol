import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { BOSS, ScriptHelper, TOKEN_IN_MINT, TOKEN_OUT_MINT } from "../utils/script-helper";

/**
 * Smoke tests for Onre program - validates core functionality
 *
 * This script builds transactions for:
 * 1. Creating test token mints and accounts
 * 2. Making a new offer
 * 3. Adding a vector to the offer
 * 4. Taking the offer (standard flow)
 * 5. Taking the offer (permissionless flow)
 * 6. Closing the offer
 *
 * Note: Transactions are built but NOT sent to chain
 */

// Test configuration
const BASE_TIME = Math.floor(Date.now() / 1000);
const BASE_PRICE = 1_100_000_000; // 1.1 (scaled by 1e9)
const APR = 36_500; // 0.0365% APR (scaled by 1e6)
const PRICE_FIX_DURATION = 60 * 60 * 24; // 1 day
const OFFER_FEE_BASIS_POINTS = 0; // 0% fee
const TOKEN_IN_AMOUNT = 1_000_000; // 1 USDC (6 decimals)
const TOKEN_OUT_AMOUNT = 1_000_000_000; // 1000 tokens (6 decimals)

async function buildSmokeTestTransactions() {
    console.log("=== Onre Program Smoke Tests ===\n");

    const helper = await ScriptHelper.create();
    const boss = await helper.getBoss();

    console.log("Program ID:", helper.program.programId.toBase58());
    console.log("Boss:", boss.toBase58());
    console.log("RPC:", helper.connection.rpcEndpoint);
    console.log();
    //
    // // Generate test keypairs
    // const user = Keypair.generate();

    console.log("Generated test keypairs:");
    // console.log("User:", user.publicKey.toBase58());
    console.log("Token In Mint:", TOKEN_IN_MINT.toBase58());
    console.log("Token Out Mint:", TOKEN_OUT_MINT.toBase58());
    console.log();

    console.log("--- Building Transactions ---\n");

    // Transaction 1: Make Offer + Add Vector + Deposit (+ optional permissionless account setup)
    console.log("Transaction 1: Setup Offer");

    // Create permissionless token accounts if needed
    const permissionlessIxs = await helper.buildCreatePermissionlessTokenAccountsIxs({
        tokenInMint: TOKEN_IN_MINT,
        tokenOutMint: TOKEN_OUT_MINT
    });

    if (permissionlessIxs.length > 0) {
        console.log(`  0. Building ${permissionlessIxs.length} permissionless token account creation instruction(s)...`);
        console.log("     ✓ Permissionless token account instructions built");
    }

    console.log("  1. Building Make Offer instruction...");
    const makeOfferIx = await helper.buildMakeOfferIx({
        tokenInMint: TOKEN_IN_MINT,
        tokenOutMint: TOKEN_OUT_MINT,
        feeBasisPoints: OFFER_FEE_BASIS_POINTS,
        needsApproval: false,
        allowPermissionless: true
    });
    console.log("     ✓ Make Offer instruction built");

    console.log("  2. Building Add Offer Vector instruction...");
    const addVectorIx = await helper.buildAddOfferVectorIx({
        tokenInMint: TOKEN_IN_MINT,
        tokenOutMint: TOKEN_OUT_MINT,
        baseTime: BASE_TIME,
        basePrice: BASE_PRICE,
        apr: APR,
        priceFixDuration: PRICE_FIX_DURATION
    });
    console.log("     ✓ Add Offer Vector instruction built");

    console.log("  3. Building Offer Vault Deposit instruction...");
    const tokenOutDepositAmount = TOKEN_OUT_AMOUNT * 2;
    const depositIx = await helper.buildOfferVaultDepositIx({
        amount: tokenOutDepositAmount,
        tokenMint: TOKEN_OUT_MINT
    });
    console.log("     ✓ Offer Vault Deposit instruction built");

    const tx1 = new Transaction();
    // Add permissionless account creation instructions first (if any)
    permissionlessIxs.forEach(ix => tx1.add(ix));
    tx1.add(makeOfferIx);
    tx1.add(addVectorIx);
    tx1.add(depositIx);
    tx1.feePayer = boss;
    tx1.recentBlockhash = (await helper.connection.getLatestBlockhash()).blockhash;

    const serializedTx1 = helper.serializeTransaction(tx1);
    console.log(`  Size: ${serializedTx1.length} bytes`);
    console.log(`  Base58: ${serializedTx1}`);
    console.log();

    // Transaction 2: Take Offer (both flows) + Close Offer
    console.log("Transaction 2: Execute & Close (3 instructions)");

    console.log("  1. Building Take Offer (Standard) instruction...");
    const takeOfferIx = await helper.buildTakeOfferIx({
        tokenInAmount: TOKEN_IN_AMOUNT,
        tokenInMint: TOKEN_IN_MINT,
        tokenOutMint: TOKEN_OUT_MINT,
        user: BOSS
    });
    console.log("     ✓ Take Offer (Standard) instruction built");

    console.log("  2. Building Take Offer (Permissionless) instruction...");
    const takeOfferPermissionlessIx = await helper.buildTakeOfferPermissionlessIx({
        tokenInAmount: TOKEN_IN_AMOUNT,
        tokenInMint: TOKEN_IN_MINT,
        tokenOutMint: TOKEN_OUT_MINT,
        user: BOSS
    });
    console.log("     ✓ Take Offer (Permissionless) instruction built");

    console.log("  3. Building Close Offer instruction...");
    const closeOfferIx = await helper.buildCloseOfferIx({
        tokenInMint: TOKEN_IN_MINT,
        tokenOutMint: TOKEN_OUT_MINT
    });
    console.log("     ✓ Close Offer instruction built");

    const tx2 = new Transaction();
    tx2.add(takeOfferIx);
    tx2.add(takeOfferPermissionlessIx);
    tx2.add(closeOfferIx);
    tx2.feePayer = boss;
    tx2.recentBlockhash = (await helper.connection.getLatestBlockhash()).blockhash;

    const serializedTx2 = helper.serializeTransaction(tx2);
    console.log(`  Size: ${serializedTx2.length} bytes`);
    console.log(`  Base58: ${serializedTx2}`);
    console.log();

    console.log("=== Smoke Test Complete ===");
    console.log("Transaction 1 (Setup):");
    console.log(serializedTx1);
    console.log();
    console.log("Transaction 2 (Execute & Close):");
    console.log(serializedTx2);
}

async function main() {
    try {
        await buildSmokeTestTransactions();
    } catch (error) {
        console.error("Smoke test failed:", error);
        process.exit(1);
    }
}

await main();
