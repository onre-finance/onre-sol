// setBoss.ts
import * as anchor from '@coral-xyz/anchor';
import { PublicKey, SystemProgram } from '@solana/web3.js';
import bs58 from 'bs58';

import { getBossAccount, initProgram, PROGRAM_ID } from './script-commons';

const NEW_BOSS = new PublicKey('9tTUg7r9ftofzoPXKeUPB35oN4Lm8KkrVDVQbbM7Xzxx'); // Replace with this

async function createSetBossTransaction() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(process.env.SOL_MAINNET_RPC_URL || '');

    const BOSS = await getBossAccount(program);

    const [statePda, _bump] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);

    try {
        const tx = await program.methods
            .setBoss(NEW_BOSS)
            .accountsPartial({
                state: statePda,
                boss: BOSS,
                systemProgram: SystemProgram.programId,
            })
            .transaction();

        tx.feePayer = BOSS;
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

await main();
