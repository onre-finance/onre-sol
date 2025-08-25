import * as anchor from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import {
    createAssociatedTokenAccountInstruction,
    getAssociatedTokenAddressSync,
    TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import bs58 from 'bs58';
import { BN } from 'bn.js';

import { getBossAccount, initProgram, PROGRAM_ID, RPC_URL } from './script-commons';

// PROD - Update these token mint addresses as needed
const TOKEN_MINT = new PublicKey('FsSJSYJKLdyxtsT25DoyLS1j2asxzBkTVuX8vtojyWob'); // USDC Mint Address

// Test
// const TOKEN_MINT = new PublicKey('qaegW5BccnepuexbHkVqcqQULqdJA115K6zHP16vR15zrcqa6r6C');  // TestUSDC Mint Address

async function createVaultWithdrawTransaction() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(RPC_URL);

    // Configuration
    const withdrawAmount = '100000000'; // 100 tokens with 6 decimals for USDC
    
    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);
    const [vaultAuthorityPda] = PublicKey.findProgramAddressSync([Buffer.from('vault_authority')], PROGRAM_ID);

    const BOSS = await getBossAccount(program);
    console.log('BOSS:', BOSS.toBase58());
    console.log('TOKEN_MINT:', TOKEN_MINT.toBase58());
    console.log('Withdraw Amount:', withdrawAmount);

    const bossTokenAccount = getAssociatedTokenAddressSync(TOKEN_MINT, BOSS, false);
    const vaultTokenAccount = getAssociatedTokenAddressSync(TOKEN_MINT, vaultAuthorityPda, true);

    console.log('Boss Token Account:', bossTokenAccount.toBase58());
    console.log('Vault Token Account:', vaultTokenAccount.toBase58());
    console.log('Vault Authority PDA:', vaultAuthorityPda.toBase58());

    // Create boss token account instruction (if it doesn't exist)
    // const createBossTokenAccountInstruction =
    //     createAssociatedTokenAccountInstruction(
    //     BOSS,
    //     bossTokenAccount,
    //     BOSS,
    //     TOKEN_MINT,
    //     TOKEN_PROGRAM_ID,
    // );

    try {
        const tx = await program.methods
            .vaultWithdraw(new BN(withdrawAmount))
            .accountsPartial({
                vaultAuthority: vaultAuthorityPda,
                tokenMint: TOKEN_MINT,
                bossTokenAccount,
                vaultTokenAccount,
                boss: BOSS,
                state: statePda,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .transaction();

        tx.feePayer = BOSS;
        tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

        const serializedTx = tx.serialize({
            requireAllSignatures: false,
            verifySignatures: false,
        });

        const base58Tx = bs58.encode(serializedTx);
        console.log('Vault Withdraw Transaction (Base58):');
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error('Error creating vault withdraw transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createVaultWithdrawTransaction();
    } catch (error) {
        console.error('Failed to create vault withdraw transaction:', error);
    }
}

await main();