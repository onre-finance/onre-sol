#!/usr/bin/env npx tsx
/**
 * Sets on-chain metadata (name, symbol, URI) for an SPL token mint.
 * - Creates metadata if it doesn't exist yet (requires mint authority = Squad)
 * - Updates metadata if it already exists (requires update authority = Squad)
 * Outputs base58 for Squad to sign and execute.
 *
 * Usage:
 *   NETWORK=devnet-test tsx scripts/utils/set-token-metadata.ts \
 *     --mint GDzG4Q7hxyqF4owhApZemQbw8gs9nPMtUfQ3e7q4kWJG \
 *     --name "Mock USDT" \
 *     --symbol MUSDT \
 *     --uri "https://arweave.net/<TX_ID>"
 */

import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
    createMetadataAccountV3,
    updateMetadataAccountV2,
    findMetadataPda,
    safeFetchMetadata,
} from "@metaplex-foundation/mpl-token-metadata";
import { publicKey as umiPublicKey } from "@metaplex-foundation/umi";
import { ScriptHelper } from "./script-helper";

const args = process.argv.slice(2);

function getArg(flag: string, defaultValue?: string): string | undefined {
    const idx = args.findIndex(a => a === flag);
    return idx !== -1 ? args[idx + 1] : defaultValue;
}

const mintAddress = getArg("--mint");
const name = getArg("--name");
const symbol = getArg("--symbol");
const uri = getArg("--uri", "");

if (!mintAddress || !name || !symbol) {
    console.error("Usage: set-token-metadata.ts --mint <address> --name <name> --symbol <symbol> [--uri <uri>]");
    process.exit(1);
}

async function main() {
    const helper = await ScriptHelper.create();
    const mint = new PublicKey(mintAddress!);
    const squad = helper.networkConfig.boss;

    const umi = createUmi(helper.networkConfig.rpcUrl);
    const umiMint = umiPublicKey(mint.toBase58());
    const umiSquad = umiPublicKey(squad.toBase58());
    const metadataPda = findMetadataPda(umi, { mint: umiMint });

    // Check if metadata account already exists
    const existingMetadata = await safeFetchMetadata(umi, metadataPda);
    const isUpdate = existingMetadata !== null;

    console.log(`\nBuilding ${isUpdate ? "update" : "create"}-metadata transaction on ${helper.networkConfig.name}...`);
    console.log(`  Mint:             ${mint.toBase58()}`);
    console.log(`  Name:             ${name}`);
    console.log(`  Symbol:           ${symbol}`);
    console.log(`  URI:              ${uri || "(empty)"}`);
    console.log(`  Signer (Squad):   ${squad.toBase58()}`);

    let umiIx;

    if (isUpdate) {
        // Update authority signs
        umiIx = updateMetadataAccountV2(umi, {
            metadata: metadataPda,
            updateAuthority: { publicKey: umiSquad, secretKey: new Uint8Array(64) } as any,
            data: {
                name: name!,
                symbol: symbol!,
                uri: uri!,
                sellerFeeBasisPoints: 0,
                creators: null,
                collection: null,
                uses: null,
            },
            primarySaleHappened: null,
            isMutable: true,
            newUpdateAuthority: umiSquad,
        });
    } else {
        // Mint authority signs
        umiIx = createMetadataAccountV3(umi, {
            metadata: metadataPda,
            mint: umiMint,
            mintAuthority: { publicKey: umiSquad, secretKey: new Uint8Array(64) } as any,
            payer: { publicKey: umiSquad, secretKey: new Uint8Array(64) } as any,
            updateAuthority: umiSquad,
            data: {
                name: name!,
                symbol: symbol!,
                uri: uri!,
                sellerFeeBasisPoints: 0,
                creators: null,
                collection: null,
                uses: null,
            },
            isMutable: true,
            collectionDetails: null,
        });
    }

    const rawIxs = umiIx.getInstructions();
    const ix = new TransactionInstruction({
        programId: new PublicKey(rawIxs[0].programId),
        keys: rawIxs[0].keys.map(k => ({
            pubkey: new PublicKey(k.pubkey),
            isSigner: k.isSigner,
            isWritable: k.isWritable,
        })),
        data: Buffer.from(rawIxs[0].data),
    });

    const tx = await helper.prepareTransaction({ ix, payer: squad });
    helper.printTransaction(tx, `${isUpdate ? "Update" : "Create"} Token Metadata`);
}

main().catch(console.error);
