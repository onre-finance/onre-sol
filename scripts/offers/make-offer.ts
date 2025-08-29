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

// PROD
const SELL_TOKEN_MINT = new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'); // USDC Mint Address
const BUY_TOKEN_MINT = new PublicKey('5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5');  // ONe Mint Address

// Test
// const SELL_TOKEN_MINT = new PublicKey('qaegW5BccnepuexbHkVqcqQUuEwgDMqCCo1wJ4fWeQu');  // TestUSDC Mint Address
// const BUY_TOKEN_MINT = new PublicKey('5Uzafw84V9rCTmYULqdJA115K6zHP16vR15zrcqa6r6C');  // TestONe  Mint Address

async function createMakeOfferOneTransaction() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(RPC_URL);

    const offerId = 1;
    const buyTokenAmount =       '20000000000000000';   // 9 decimals for ONe
    const sellTokenStartAmount = '20160514420000';      // 6 decimals for USDC
    const sellTokenEndAmount =   '20340430000000';      // 6 decimals for USDC
    const offerStartTime = Math.floor(new Date(Date.UTC(2025, 4, 27, 0, 0, 0)).getTime() / 1000); // May 27, 2025
    const offerEndTime = offerStartTime + (60 * 60 * 24 * 19); // +19 days
    const priceFixDuration = 60 * 60 * 24; // 1 day

    const [offerAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer_authority'), new BN(offerId).toArrayLike(Buffer, 'le', 8)],
        program.programId,
    );

    console.log('programId:', program.programId.toBase58());
    console.log('offerAuthority:', offerAuthority.toBase58());

    const BOSS = await getBossAccount(program);
    console.log('BOSS:', BOSS.toBase58());

    // TODO: check if boss sell token account exists, create if it doesn't
    const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(
        BOSS,
        getAssociatedTokenAddressSync(SELL_TOKEN_MINT, offerAuthority, true),
        offerAuthority,
        SELL_TOKEN_MINT,
        TOKEN_PROGRAM_ID,
    );
    const offerBuyTokenAccountInstruction = createAssociatedTokenAccountInstruction(
        BOSS,
        getAssociatedTokenAddressSync(BUY_TOKEN_MINT, offerAuthority, true),
        offerAuthority,
        BUY_TOKEN_MINT,
        TOKEN_PROGRAM_ID
    );
    // Derive the state PDA
    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);

    const [offerPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer'), new BN(offerId).toArrayLike(Buffer, 'le', 8)],
        PROGRAM_ID,
    );
    try {
        const tx = await program.methods
            .makeOfferOne(
              new BN(offerId), 
              new BN(buyTokenAmount),
              new BN(sellTokenStartAmount),
              new BN(sellTokenEndAmount),
              new BN(offerStartTime),
              new BN(offerEndTime),
              new BN(priceFixDuration))
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: getAssociatedTokenAddressSync(SELL_TOKEN_MINT, offerAuthority, true),
                offerBuyToken1Account: getAssociatedTokenAddressSync(BUY_TOKEN_MINT, offerAuthority, true),
                offerTokenAuthority: offerAuthority,
                bossBuyToken1Account: getAssociatedTokenAddressSync(BUY_TOKEN_MINT, BOSS, true),
                sellTokenMint: SELL_TOKEN_MINT,
                buyToken1Mint: BUY_TOKEN_MINT,
                state: statePda,
                boss: BOSS,
            })
            .preInstructions([offerSellTokenAccountInstruction, offerBuyTokenAccountInstruction ])
            .transaction();

        tx.feePayer = BOSS;
        tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

        const serializedTx = tx.serialize({
            requireAllSignatures: false,
            verifySignatures: false,
        });

        const base58Tx = bs58.encode(serializedTx);
        console.log('Make Offer Transaction (Base58):');
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error('Error creating transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createMakeOfferOneTransaction();
    } catch (error) {
        console.error('Failed to create make offer transaction:', error);
    }
}

await main();
