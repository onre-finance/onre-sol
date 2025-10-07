import { Keypair, PublicKey, Transaction, TransactionInstruction } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

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
const TOKEN_IN_MINT = new PublicKey(""); // USDC-like (6 decimals)
const TOKEN_OUT_MINT = new PublicKey("TODO"); // ONyc-like (9 decimals)
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

    // Generate test keypairs
    const user = Keypair.generate();

    console.log("Generated test keypairs:");
    console.log("User:", user.publicKey.toBase58());
    console.log("Token In Mint:", TOKEN_IN_MINT.toBase58());
    console.log("Token Out Mint:", TOKEN_OUT_MINT.toBase58());
    console.log();

    const instructions: TransactionInstruction[] = [];

    // Note: In a real devnet deployment, you would need to:
    // 1. Airdrop SOL to payer and user accounts
    // 2. Create token mints
    // 3. Create token accounts for boss and user
    // 4. Mint tokens to boss's token out account
    // 5. Mint tokens to user's token in account
    // These setup steps are omitted as they require actual on-chain execution

    console.log("--- Building Transactions ---\n");

    // Transaction 1: Make Offer
    console.log("1. Building Make Offer instruction...");
    const makeOfferIx = await helper.buildMakeOfferIx({
        tokenInMint: TOKEN_IN_MINT,
        tokenOutMint: TOKEN_OUT_MINT,
        feeBasisPoints: OFFER_FEE_BASIS_POINTS,
        needsApproval: false,
        allowPermissionless: true // Enable permissionless flow for testing
    });
    instructions.push(makeOfferIx);
    console.log("   ✓ Make Offer instruction built");
    console.log();

    // Transaction 2: Add Offer Vector
    console.log("2. Building Add Offer Vector instruction...");
    const addVectorIx = await helper.buildAddOfferVectorIx({
        tokenInMint: TOKEN_IN_MINT,
        tokenOutMint: TOKEN_OUT_MINT,
        baseTime: BASE_TIME,
        basePrice: BASE_PRICE,
        apr: APR,
        priceFixDuration: PRICE_FIX_DURATION
    });
    instructions.push(addVectorIx);
    console.log("   ✓ Add Offer Vector instruction built");
    console.log(`   Base Time: ${new Date(BASE_TIME * 1000).toISOString()}`);
    console.log(`   Base Price: ${BASE_PRICE / 1e9}`);
    console.log(`   APR: ${APR / 1e6}%`);
    console.log();

    // Transaction 3: Deposit to Offer Vault
    console.log("3. Building Offer Vault Deposit instruction...");
    const tokenOutDepositAmount = TOKEN_OUT_AMOUNT * 2;
    const depositIx = await helper.buildOfferVaultDepositIx({
        amount: tokenOutDepositAmount,
        tokenMint: TOKEN_OUT_MINT
    });
    instructions.push(depositIx);
    console.log("   ✓ Offer Vault Deposit instruction built");
    console.log(`   Amount: ${tokenOutDepositAmount / 1e9} tokens`);
    console.log();

    // Transaction 4: Take Offer (Standard Flow)
    console.log("4. Building Take Offer (Standard) instruction...");
    const takeOfferIx = await helper.buildTakeOfferIx({
        tokenInAmount: TOKEN_IN_AMOUNT,
        tokenInMint: TOKEN_IN_MINT,
        tokenOutMint: TOKEN_OUT_MINT,
        user: user.publicKey
    });
    instructions.push(takeOfferIx);
    console.log("   ✓ Take Offer (Standard) instruction built");
    console.log(`   User: ${user.publicKey.toBase58()}`);
    console.log(`   Token In Amount: ${TOKEN_IN_AMOUNT / 1e6} USDC`);
    console.log();

    // Transaction 5: Take Offer (Permissionless Flow)
    console.log("5. Building Take Offer (Permissionless) instruction...");
    const takeOfferPermissionlessIx = await helper.buildTakeOfferPermissionlessIx({
        tokenInAmount: TOKEN_IN_AMOUNT,
        tokenInMint: TOKEN_IN_MINT,
        tokenOutMint: TOKEN_OUT_MINT,
        user: user.publicKey
    });
    instructions.push(takeOfferPermissionlessIx);
    console.log("   ✓ Take Offer (Permissionless) instruction built");
    console.log(`   User: ${user.publicKey.toBase58()}`);
    console.log(`   Token In Amount: ${TOKEN_IN_AMOUNT / 1e6} USDC`);
    console.log();

    // Transaction 6: Close Offer
    console.log("6. Building Close Offer instruction...");
    const closeOfferIx = await helper.buildCloseOfferIx({
        tokenInMint: TOKEN_IN_MINT,
        tokenOutMint: TOKEN_OUT_MINT
    });
    instructions.push(closeOfferIx);
    console.log("   ✓ Close Offer instruction built");
    console.log();

    // Build transaction
    const tx = new Transaction();
    instructions.forEach(ix => {
        tx.add(ix);
    });

    console.log("--- Transaction Summaries ---\n");
    console.log(`  Instructions: ${tx.instructions.length}`);
    console.log(`  Fee Payer: ${tx.feePayer?.toBase58()}`);
    console.log(`  Recent Blockhash: ${tx.recentBlockhash}`);
    const serialized = helper.serializeTransaction(tx);
    console.log(`  Size: ${serialized.length} bytes (base58)`);
    console.log(`  Serialized (Base58): ${serialized}`);
    console.log();

    console.log("=== Smoke Test Complete ===");
    console.log(`Built ${instructions.length} transactions successfully`);
    console.log("Note: Transactions are NOT sent to chain");

    return instructions;
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
