import * as anchor from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';
import { BN } from 'bn.js';

import { getBossAccount, initProgram, PROGRAM_ID, RPC_URL } from '../utils/script-commons';

// PROD - Update these token mint addresses as needed
const TOKEN_IN_MINT = new PublicKey('FsSJSYJKLdyxtsT25DoyLS1j2asxzBkTVuX8vtojyWob'); // USDC Mint Address (input token)
const TOKEN_OUT_MINT_1 = new PublicKey('FsSJSYJKLdyxtsT25DoyLS1j2asxzBkTVuX8vtojyWob'); // ONe Mint Address (output token 1)
const TOKEN_OUT_MINT_2 = new PublicKey('FsSJSYJKLdyxtsT25DoyLS1j2asxzBkTVuX8vtojyWob'); // SOL Mint Address (output token 2)

// Test
// const TOKEN_IN_MINT = new PublicKey('qaegW5BccnepuexbHkVqcqQULqdJA115K6zHP16vR15zrcqa6r6C');  // TestUSDC
// const TOKEN_OUT_MINT_1 = new PublicKey('5Uzafw84V9rCTmYULqdJA115K6zHP16vR15zrcqa6r6C');  // TestONe
// const TOKEN_OUT_MINT_2 = new PublicKey('So11111111111111111111111111111111111111112'); // SOL

async function createMakeDualRedemptionOfferTransaction() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(RPC_URL);

    // Configuration - Adjust these values as needed
    const startTime = new BN(Math.floor(Date.now() / 1000) + 60); // Start in 1 minute
    const endTime = new BN(startTime.toNumber() + (60 * 60 * 24 * 7)); // End in 1 week
    
    // ⚠️  CRITICAL: Prices MUST be in 9 decimals precision or calculations will break!
    // These represent how many output tokens you get per 1 input token
    // Example: 2000000000 = 2.0 output tokens per 1 input token (with 9 decimal places)
    const price1 = new BN('2000000000'); // 2.0 tokens for output token 1 (MUST BE 9 DECIMALS)
    const price2 = new BN('1500000000'); // 1.5 tokens for output token 2 (MUST BE 9 DECIMALS)
    const ratioBasisPoints = new BN(8000); // 80% goes to token_out_1, 20% to token_out_2

    console.log('Creating dual redemption offer with:');
    console.log('TOKEN_IN_MINT:', TOKEN_IN_MINT.toBase58());
    console.log('TOKEN_OUT_MINT_1:', TOKEN_OUT_MINT_1.toBase58());
    console.log('TOKEN_OUT_MINT_2:', TOKEN_OUT_MINT_2.toBase58());
    console.log('Start Time:', new Date(startTime.toNumber() * 1000).toISOString());
    console.log('End Time:', new Date(endTime.toNumber() * 1000).toISOString());
    console.log('Price 1 (raw):', price1.toString());
    console.log('Price 1 (human-readable):', (price1.toNumber() / 1e9).toFixed(9), 'output tokens per 1 input token');
    console.log('Price 2 (raw):', price2.toString());
    console.log('Price 2 (human-readable):', (price2.toNumber() / 1e9).toFixed(9), 'output tokens per 1 input token');
    console.log('Ratio Basis Points (80/20):', ratioBasisPoints.toString());
    console.log('⚠️  Prices are in 9 decimal precision - DO NOT change this format!');

    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);

    const BOSS = await getBossAccount(program);
    console.log('BOSS:', BOSS.toBase58());

    try {
        const tx = await program.methods
            .makeDualRedemptionOffer(
                startTime,
                endTime,
                price1,
                price2,
                ratioBasisPoints
            )
            .accountsPartial({
                tokenInMint: TOKEN_IN_MINT,
                tokenOutMint1: TOKEN_OUT_MINT_1,
                tokenOutMint2: TOKEN_OUT_MINT_2,
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
        console.log('Make Dual Redemption Offer Transaction (Base58):');
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error('Error creating make dual redemption offer transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createMakeDualRedemptionOfferTransaction();
    } catch (error) {
        console.error('Failed to create make dual redemption offer transaction:', error);
    }
}

await main();