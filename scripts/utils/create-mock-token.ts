#!/usr/bin/env npx tsx
/**
 * Creates a mock SPL token mint in two steps:
 *   1. Local wallet creates the mint and pays rent (broadcasts directly)
 *   2. Outputs a base58 transaction for Squad to transfer mint authority to itself
 *
 * Usage:
 *   NETWORK=devnet-test tsx scripts/utils/create-mock-token.ts --decimals 6 --symbol USDT
 *
 * After Squad executes step 2, paste the mint address into network-config.ts.
 */

import { Keypair, sendAndConfirmTransaction, SystemProgram, Transaction } from "@solana/web3.js";
import {
    createInitializeMint2Instruction,
    createSetAuthorityInstruction,
    AuthorityType,
    getMinimumBalanceForRentExemptMint,
    MINT_SIZE,
    TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { ScriptHelper } from "./script-helper";

const args = process.argv.slice(2);

function getArg(flag: string, defaultValue?: string): string | undefined {
    const idx = args.findIndex(a => a === flag);
    return idx !== -1 ? args[idx + 1] : defaultValue;
}

const decimals = parseInt(getArg("--decimals", "6")!, 10);
const symbol = getArg("--symbol", "TOKEN")!;

async function main() {
    const helper = await ScriptHelper.create();
    const payer = helper.wallet.payer;
    const boss = helper.networkConfig.boss;

    console.log(`\nCreating mock ${symbol} mint on ${helper.networkConfig.name}...`);
    console.log(`  Decimals: ${decimals}`);
    console.log(`  Payer:    ${payer.publicKey.toBase58()} (local wallet)`);
    console.log(`  Squad:    ${boss.toBase58()}`);

    // ── Step 1: Create mint with local wallet as temporary mint authority ──
    const mintKeypair = Keypair.generate();
    const lamports = await getMinimumBalanceForRentExemptMint(helper.connection);

    const createTx = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: mintKeypair.publicKey,
            space: MINT_SIZE,
            lamports,
            programId: TOKEN_PROGRAM_ID,
        }),
        createInitializeMint2Instruction(
            mintKeypair.publicKey,
            decimals,
            payer.publicKey,  // temporary mint authority (local wallet)
            payer.publicKey,  // temporary freeze authority (local wallet)
        ),
    );

    console.log(`\n[Step 1] Creating mint account...`);
    const sig = await sendAndConfirmTransaction(helper.connection, createTx, [payer, mintKeypair]);
    console.log(`  Mint:      ${mintKeypair.publicKey.toBase58()}`);
    console.log(`  Signature: ${sig}`);

    // ── Step 2: Transfer both authorities to Squad (local wallet signs and broadcasts) ──
    const transferTx = new Transaction().add(
        createSetAuthorityInstruction(
            mintKeypair.publicKey,
            payer.publicKey,
            AuthorityType.MintTokens,
            boss,
        ),
        createSetAuthorityInstruction(
            mintKeypair.publicKey,
            payer.publicKey,
            AuthorityType.FreezeAccount,
            boss,
        ),
    );

    console.log(`\n[Step 2] Transferring mint and freeze authority to Squad...`);
    const transferSig = await sendAndConfirmTransaction(helper.connection, transferTx, [payer]);
    console.log(`  Authority transferred to: ${boss.toBase58()}`);
    console.log(`  Signature: ${transferSig}`);

    console.log(`\nAdd to network-config.ts:`);
    console.log(`  const MOCK_${symbol}_DEVNET = new PublicKey("${mintKeypair.publicKey.toBase58()}");`);
}

main().catch(console.error);
