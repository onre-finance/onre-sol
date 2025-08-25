import * as anchor from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';

import { getBossAccount, initProgram, PROGRAM_ID, RPC_URL } from './script-commons';

async function createInitializeOffersTransaction() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(RPC_URL);

    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);

    const BOSS = await getBossAccount(program);
    console.log('BOSS:', BOSS.toBase58());
    console.log('State PDA:', statePda.toBase58());
    console.log('Program ID:', PROGRAM_ID.toBase58());

    console.log('Initializing offers accounts...');

    try {
        const tx = await program.methods
            .initializeOffers()
            .accountsPartial({
                state: statePda,
                boss: BOSS,
            })
            .transaction();

        tx.feePayer = BOSS;
        tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

        const serializedTx = tx.serialize({
            requireAllSignatures: false,
            verifySignatures: false,
        });

        const base58Tx = bs58.encode(serializedTx);
        console.log('Initialize Offers Transaction (Base58):');
        console.log(base58Tx);

        console.log('\nThis transaction will initialize:');
        console.log('- Buy offers account');
        console.log('- Single redemption offers account');
        console.log('- Dual redemption offers account');

        return base58Tx;
    } catch (error) {
        console.error('Error creating initialize offers transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createInitializeOffersTransaction();
    } catch (error) {
        console.error('Failed to create initialize offers transaction:', error);
    }
}

await main();