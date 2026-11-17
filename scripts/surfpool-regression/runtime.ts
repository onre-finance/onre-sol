import anchor from "@anchor-lang/core";
import { AnchorProvider, Program } from "@anchor-lang/core";
import BN from "bn.js";
import {
    AddressLookupTableAccount,
    AddressLookupTableProgram,
    ComputeBudgetProgram,
    Connection,
    Keypair,
    PublicKey,
    sendAndConfirmTransaction,
    Transaction,
    TransactionInstruction,
    TransactionMessage,
    VersionedTransaction,
} from "@solana/web3.js";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import idl from "../../target/idl/onreapp.json";
import { Onreapp } from "../../target/types/onreapp";
import { LOCAL_RPC_URL, PROGRAM_ID } from "./constants";

// Native ESM cannot discover Anchor's CommonJS Wallet re-export.
const { Wallet } = anchor;
type Wallet = anchor.Wallet;

export interface SmokeRuntime {
    connection: Connection;
    provider: AnchorProvider;
    program: Program<Onreapp>;
    authority: Keypair;
}

export function resolveAuthorityPath(): string {
    const configured =
        process.env.SURFPOOL_REGRESSION_UPGRADE_AUTHORITY_KEYPAIR ??
        process.env.SURFPOOL_UPGRADE_AUTHORITY_KEYPAIR ??
        process.env.UPGRADE_AUTHORITY_KEYPAIR ??
        process.env.ANCHOR_WALLET ??
        "~/.config/solana/id.json";
    return configured.replace(/^~/, os.homedir());
}

export function loadKeypair(filePath: string): Keypair {
    return Keypair.fromSecretKey(new Uint8Array(JSON.parse(fs.readFileSync(filePath, "utf8"))));
}

export function createRuntime(authorityPath = resolveAuthorityPath()): SmokeRuntime {
    const authority = loadKeypair(authorityPath);
    const connection = new Connection(LOCAL_RPC_URL, "confirmed");
    const wallet = new Wallet(authority);
    const provider = new AnchorProvider(connection, wallet, { commitment: "confirmed" });
    const idlCopy = JSON.parse(JSON.stringify(idl));
    idlCopy.address = PROGRAM_ID.toBase58();
    const program = new Program<Onreapp>(idlCopy, provider);
    anchor.setProvider(provider);
    return { connection, provider, program, authority };
}

export async function sendIxs(runtime: SmokeRuntime, label: string, ixs: TransactionInstruction[], signers: Keypair[] = []): Promise<string> {
    const tx = new Transaction().add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }), ...ixs);
    tx.feePayer = runtime.authority.publicKey;
    tx.recentBlockhash = (await runtime.connection.getLatestBlockhash()).blockhash;
    try {
        const signature = await sendAndConfirmTransaction(runtime.connection, tx, [runtime.authority, ...signers], {
            commitment: "confirmed",
            skipPreflight: false,
        });
        console.log(`  ok ${label}: ${signature}`);
        return signature;
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (!message.includes("Transaction too large")) {
            throw error;
        }
        const signature = await sendIxsV0(runtime, label, ixs, signers);
        console.log(`  ok ${label} (v0 lookup): ${signature}`);
        return signature;
    }
}

async function sendIxsV0(runtime: SmokeRuntime, label: string, ixs: TransactionInstruction[], signers: Keypair[]): Promise<string> {
    const lookup = await createLookupTableForInstructions(runtime, label, ixs, signers);
    const { blockhash } = await runtime.connection.getLatestBlockhash();
    const message = new TransactionMessage({
        payerKey: runtime.authority.publicKey,
        recentBlockhash: blockhash,
        instructions: [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }), ...ixs],
    }).compileToV0Message([lookup]);
    const tx = new VersionedTransaction(message);
    tx.sign([runtime.authority, ...signers]);
    const signature = await runtime.connection.sendTransaction(tx, { skipPreflight: false });
    await runtime.connection.confirmTransaction(signature, "confirmed");
    return signature;
}

async function createLookupTableForInstructions(runtime: SmokeRuntime, label: string, ixs: TransactionInstruction[], signers: Keypair[]): Promise<AddressLookupTableAccount> {
    const signerKeys = new Set([runtime.authority.publicKey.toBase58(), ...signers.map((signer) => signer.publicKey.toBase58())]);
    const addresses = dedupePubkeys(ixs.flatMap((ix) => [ix.programId, ...ix.keys.map((key) => key.pubkey)]).filter((key) => !signerKeys.has(key.toBase58())));

    const recentSlot = await runtime.connection.getSlot("confirmed");
    const [createIx, lookupTable] = AddressLookupTableProgram.createLookupTable({
        authority: runtime.authority.publicKey,
        payer: runtime.authority.publicKey,
        recentSlot,
    });
    await sendAndConfirmTransaction(runtime.connection, new Transaction().add(createIx), [runtime.authority], { commitment: "confirmed", skipPreflight: false });

    for (let i = 0; i < addresses.length; i += 24) {
        const extendIx = AddressLookupTableProgram.extendLookupTable({
            payer: runtime.authority.publicKey,
            authority: runtime.authority.publicKey,
            lookupTable,
            addresses: addresses.slice(i, i + 24),
        });
        await sendAndConfirmTransaction(runtime.connection, new Transaction().add(extendIx), [runtime.authority], { commitment: "confirmed", skipPreflight: false });
    }

    const lookupAccount = (await runtime.connection.getAddressLookupTable(lookupTable)).value;
    if (!lookupAccount) {
        throw new Error(`failed to create lookup table for ${label}`);
    }
    return lookupAccount;
}

function dedupePubkeys(keys: PublicKey[]): PublicKey[] {
    const seen = new Set<string>();
    const result: PublicKey[] = [];
    for (const key of keys) {
        const value = key.toBase58();
        if (seen.has(value)) {
            continue;
        }
        seen.add(value);
        result.push(key);
    }
    return result;
}

export async function fetchNullable<T>(fetcher: () => Promise<T>): Promise<T | null> {
    try {
        return await fetcher();
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (message.includes("Account does not exist") || message.includes("Could not find") || message.includes("AccountNotFound")) {
            return null;
        }
        throw error;
    }
}

export function requireAccount<T>(value: T | null, label: string): T {
    if (value == null) {
        throw new Error(`${label} was not found on the fork`);
    }
    return value;
}

export function bn(value: number | bigint | string): BN {
    return new BN(value.toString());
}

export function tokenAmount(uiAmount: number, decimals = 6): number {
    return Math.round(uiAmount * 10 ** decimals);
}

export function repoPath(relativePath: string): string {
    return path.resolve(process.cwd(), relativePath);
}
