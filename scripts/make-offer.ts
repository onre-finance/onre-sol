import * as anchor from '@coral-xyz/anchor';
import {AnchorProvider, Program} from '@coral-xyz/anchor';
import {PublicKey, SystemProgram} from '@solana/web3.js';
import type {OnreApp} from '../target/types/onre_app';
import {
    createAssociatedTokenAccountInstruction,
    getAssociatedTokenAddressSync,
    TOKEN_PROGRAM_ID
} from '@solana/spl-token'
import bs58 from 'bs58';
import {BN} from 'bn.js'

import idl from "../target/idl/onre_app.json" assert {type: "json"};

const PROGRAM_ID = new PublicKey('J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2');

const BOSS = new PublicKey('7rzEKejyAXJXMkGfRhMV9Vg1k7tFznBBEFu3sfLNz8LC');

const SELL_TOKEN_MINT = new PublicKey("qaegW5BccnepuexbHkVqcqQUuEwgDMqCCo1wJ4fWeQu");

const BUY_TOKEN_MINT = new PublicKey("5Uzafw84V9rCTmYULqdJA115K6zHP16vR15zrcqa6r6C")




async function createMakeOfferOneTransaction() {
    const offerId = new BN(2);
    const buyTokenAmount = new BN(100e9);
    const sellTokenAmount = new BN(100e9);
    const connection = new anchor.web3.Connection('https://api.mainnet-beta.solana.com');
    const wallet = new anchor.Wallet(anchor.web3.Keypair.generate());
    const provider = new AnchorProvider(connection, wallet);
    const program = new Program(idl as OnreApp, provider);
    anchor.setProvider(provider);

    const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], program.programId);

    const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(
        BOSS,
        getAssociatedTokenAddressSync(SELL_TOKEN_MINT, offerAuthority, true),
        offerAuthority,
        SELL_TOKEN_MINT,
        TOKEN_PROGRAM_ID
    );
    const offerBuyTokenAccountInstruction = createAssociatedTokenAccountInstruction(
        BOSS,
        getAssociatedTokenAddressSync(BUY_TOKEN_MINT, offerAuthority, true),
        offerAuthority,
        BUY_TOKEN_MINT,
    );
    // Derive the state PDA
    const [statePda] = PublicKey.findProgramAddressSync(
        [Buffer.from('state')],
        PROGRAM_ID
    );

    const [offerPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer'),
            offerId.toArrayLike(Buffer, 'le', 8)], PROGRAM_ID
    )
    try {
        const tx = await program.methods
            .makeOfferOne(offerId, buyTokenAmount, sellTokenAmount)
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: getAssociatedTokenAddressSync(SELL_TOKEN_MINT, offerAuthority, true),
                offerBuyToken1Account: getAssociatedTokenAddressSync(BUY_TOKEN_MINT, offerAuthority, true),
                offerTokenAuthority: offerAuthority,
                bossBuyToken1Account: getAssociatedTokenAddressSync(BUY_TOKEN_MINT, BOSS, true),
                sellTokenMint: SELL_TOKEN_MINT,
                buyToken1Mint: BUY_TOKEN_MINT,
                state: statePda,
                boss: BOSS
            })
            .preInstructions([offerBuyTokenAccountInstruction, offerSellTokenAccountInstruction])
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

main();