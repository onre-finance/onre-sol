import * as anchor from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';

import { getBossAccount, initProgram, PROGRAM_ID, RPC_URL } from './script-commons';

async function createInitializeVaultTransaction() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(RPC_URL);

    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);
    const [vaultAuthorityPda] = PublicKey.findProgramAddressSync([Buffer.from('vault_authority')], PROGRAM_ID);

    const BOSS = await getBossAccount(program);
    console.log('BOSS:', BOSS.toBase58());
    console.log('State PDA:', statePda.toBase58());
    console.log('Vault Authority PDA:', vaultAuthorityPda.toBase58());
    console.log('Program ID:', PROGRAM_ID.toBase58());

    console.log('Initializing vault authority...');

    try {
        const tx = await program.methods
            .initializeVaultAuthority()
            .accountsPartial({
                state: statePda,
                vaultAuthority: vaultAuthorityPda,
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
        console.log('Initialize Vault Authority Transaction (Base58):');
        console.log(base58Tx);

        console.log('\nThis transaction will initialize:');
        console.log('- Vault authority PDA for managing program-owned token accounts');
        console.log('- Required for vault deposit/withdraw operations');

        return base58Tx;
    } catch (error) {
        console.error('Error creating initialize vault transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createInitializeVaultTransaction();
    } catch (error) {
        console.error('Failed to create initialize vault transaction:', error);
    }
}

await main();