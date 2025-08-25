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

async function createVaultDepositTransaction() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(RPC_URL);

    // Configuration
    const depositAmount = '1000000000'; // 1000 tokens with 6 decimals for USDC
    
    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);
    const [vaultAuthorityPda] = PublicKey.findProgramAddressSync([Buffer.from('vault_authority')], PROGRAM_ID);

    const BOSS = await getBossAccount(program);
    console.log('BOSS:', BOSS.toBase58());
    console.log('TOKEN_MINT:', TOKEN_MINT.toBase58());
    console.log('Deposit Amount:', depositAmount);

    const bossTokenAccount = getAssociatedTokenAddressSync(TOKEN_MINT, BOSS, false);
    const vaultTokenAccount = getAssociatedTokenAddressSync(TOKEN_MINT, vaultAuthorityPda, true);

    console.log('Boss Token Account:', bossTokenAccount.toBase58());
    console.log('Vault Token Account:', vaultTokenAccount.toBase58());
    console.log('Vault Authority PDA:', vaultAuthorityPda.toBase58());

    // Create vault token account instruction (if it doesn't exist)
    const createVaultTokenAccountInstruction = createAssociatedTokenAccountInstruction(
        BOSS,
        vaultTokenAccount,
        vaultAuthorityPda,
        TOKEN_MINT,
        TOKEN_PROGRAM_ID,
    );

    try {
        const tx = await program.methods
            .vaultDeposit(new BN(depositAmount))
            .accountsPartial({
                tokenMint: TOKEN_MINT,
                state: statePda,
                boss: BOSS,
                bossTokenAccount,
                vaultTokenAccount,
                vaultAuthority: vaultAuthorityPda,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .preInstructions([createVaultTokenAccountInstruction])
            .transaction();

        tx.feePayer = BOSS;
        tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

        const serializedTx = tx.serialize({
            requireAllSignatures: false,
            verifySignatures: false,
        });

        const base58Tx = bs58.encode(serializedTx);
        console.log('Vault Deposit Transaction (Base58):');
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error('Error creating vault deposit transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createVaultDepositTransaction();
    } catch (error) {
        console.error('Failed to create vault deposit transaction:', error);
    }
}

await main();