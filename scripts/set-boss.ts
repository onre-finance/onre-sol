// setBoss.ts
import * as anchor from '@coral-xyz/anchor';
import {AnchorProvider, Program} from '@coral-xyz/anchor';
import {PublicKey, SystemProgram} from '@solana/web3.js';
import type {OnreApp} from '../target/types/onre_app';
import bs58 from 'bs58';

import idl from "../target/idl/onre_app.json" assert { type: "json" };

const PROGRAM_ID = new PublicKey('J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2');

const CURRENT_BOSS = new PublicKey('7rzEKejyAXJXMkGfRhMV9Vg1k7tFznBBEFu3sfLNz8LC');

const NEW_BOSS = new PublicKey('9tTUg7r9ftofzoPXKeUPB35oN4Lm8KkrVDVQbbM7Xzxx'); // Replace this

async function createSetBossTransaction() {
    const connection = new anchor.web3.Connection('https://api.mainnet-beta.solana.com');
    const wallet = new anchor.Wallet(anchor.web3.Keypair.generate());
    const provider = new AnchorProvider(connection, wallet);
    const program = new Program(idl as OnreApp, provider);
    anchor.setProvider(provider);


    // Derive the state PDA
    const [statePda, _bump] = PublicKey.findProgramAddressSync(
        [Buffer.from('state')],
        PROGRAM_ID
    );

    try {
        const tx = await program.methods
            .setBoss(NEW_BOSS)
            .accountsPartial({
                state: statePda,
                boss: CURRENT_BOSS,
                systemProgram: SystemProgram.programId,
            })
            .transaction();

        tx.feePayer = CURRENT_BOSS;
        tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

        const serializedTx = tx.serialize({
            requireAllSignatures: false,
            verifySignatures: false,
        });

        const base58Tx = bs58.encode(serializedTx);
        console.log('Set Boss Transaction (Base58):');
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error('Error creating transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createSetBossTransaction();
    } catch (error) {
        console.error('Failed to create set boss transaction:', error);
    }
}

main();