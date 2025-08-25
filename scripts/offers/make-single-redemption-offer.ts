import * as anchor from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';
import { BN } from 'bn.js';

import { getBossAccount, initProgram, PROGRAM_ID, RPC_URL } from '../utils/script-commons';

// PROD - Update these token mint addresses as needed
const TOKEN_IN_MINT = new PublicKey('FsSJSYJKLdyxtsT25DoyLS1j2asxzBkTVuX8vtojyWob'); // USDC Mint Address (input token)
const TOKEN_OUT_MINT = new PublicKey('FsSJSYJKLdyxtsT25DoyLS1j2asxzBkTVuX8vtojyWob'); // ONe Mint Address (output token)

// Test
// const TOKEN_IN_MINT = new PublicKey('qaegW5BccnepuexbHkVqcqQULqdJA115K6zHP16vR15zrcqa6r6C');  // TestUSDC
// const TOKEN_OUT_MINT = new PublicKey('5Uzafw84V9rCTmYULqdJA115K6zHP16vR15zrcqa6r6C');  // TestONe

async function createMakeSingleRedemptionOfferTransaction() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(RPC_URL);

    // Configuration - Adjust these values as needed
    const startTime = new BN(Math.floor(Date.now() / 1000) + 60); // Start in 1 minute
    const endTime = new BN(startTime.toNumber() + (60 * 60 * 24 * 7)); // End in 1 week
    
    // ⚠️  CRITICAL: Price MUST be in 9 decimals precision or calculations will break!
    // This represents how many output tokens you get per 1 input token
    // Example: 2000000000 = 2.0 output tokens per 1 input token (with 9 decimal places)
    const price = new BN('2000000000'); // 2.0 tokens (MUST BE 9 DECIMALS)

    console.log('Creating single redemption offer with:');
    console.log('TOKEN_IN_MINT:', TOKEN_IN_MINT.toBase58());
    console.log('TOKEN_OUT_MINT:', TOKEN_OUT_MINT.toBase58());
    console.log('Start Time:', new Date(startTime.toNumber() * 1000).toISOString());
    console.log('End Time:', new Date(endTime.toNumber() * 1000).toISOString());
    console.log('Price (raw):', price.toString());
    console.log('Price (human-readable):', (price.toNumber() / 1e9).toFixed(9), 'output tokens per 1 input token');
    console.log('⚠️  Price is in 9 decimal precision - DO NOT change this format!');

    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);

    const BOSS = await getBossAccount(program);
    console.log('BOSS:', BOSS.toBase58());

    try {
        const tx = await program.methods
            .makeSingleRedemptionOffer(
                startTime,
                endTime,
                price
            )
            .accountsPartial({
                tokenInMint: TOKEN_IN_MINT,
                tokenOutMint: TOKEN_OUT_MINT,
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
        console.log('Make Single Redemption Offer Transaction (Base58):');
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error('Error creating make single redemption offer transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createMakeSingleRedemptionOfferTransaction();
    } catch (error) {
        console.error('Failed to create make single redemption offer transaction:', error);
    }
}

await main();