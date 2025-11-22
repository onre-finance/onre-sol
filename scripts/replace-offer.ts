import * as anchor from '@coral-xyz/anchor';
import {
    createAssociatedTokenAccountInstruction,
    getAssociatedTokenAddressSync,
    TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import bs58 from 'bs58';
import { BN } from 'bn.js';

import { getBossAccount, getOffer, initProgram, PROGRAM_ID, RPC_URL } from './script-commons';
import { PublicKey } from '@solana/web3.js';

async function createMakeOfferOneTransaction() {
    const oldOfferId = new BN(1);
    const offerId = new BN(1);

    const buyTokenAmount =       new BN('20000000000000000');                   // 9 decimals for ONyc
    const sellTokenStartAmount = new BN('20988976000000');                      // 6 decimals for USDC
    const sellTokenEndAmount =   new BN('21191718880000');                      // 6 decimals for USDC
    const offerStartTime = Math.floor(new Date(2025, 10, 21).getTime() / 1000); // November 21, 2025
    const offerEndTime = offerStartTime + (60 * 60 * 24 * 28);                  // +28 days (December 2025, 19th)
    const priceFixDuration = new BN(60 * 60 * 24); // 1 day

    const program = await initProgram();
    const connection = new anchor.web3.Connection(RPC_URL);

    const BOSS = await getBossAccount(program);
    const offer = await getOffer(oldOfferId, program);

    const [oldOfferAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer_authority'), oldOfferId.toArrayLike(Buffer, 'le', 8)],
        program.programId,
    );
    const [offerAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)],
        program.programId,
    );

    const offerSellTokenAccountInstruction = createAssociatedTokenAccountInstruction(
        BOSS,
        getAssociatedTokenAddressSync(offer.sellTokenMint, offerAuthority, true),
        offerAuthority,
        offer.sellTokenMint,
        TOKEN_PROGRAM_ID,
    );
    const offerBuyTokenAccountInstruction = createAssociatedTokenAccountInstruction(
        BOSS,
        getAssociatedTokenAddressSync(offer.buyToken1.mint, offerAuthority, true),
        offerAuthority,
        offer.buyToken1.mint,
    );
    // Derive the state PDA
    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);

    const [offerPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)],
        PROGRAM_ID,
    );

    const [oldOfferPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer'), oldOfferId.toArrayLike(Buffer, 'le', 8)],
        PROGRAM_ID,
    );
    try {
        let closeInstruction = await program.methods
            .closeOfferOne()
            .accountsPartial({
                offer: oldOfferPda,
                offerSellTokenAccount: getAssociatedTokenAddressSync(offer.sellTokenMint, oldOfferAuthority, true),
                offerBuy1TokenAccount: getAssociatedTokenAddressSync(offer.buyToken1.mint, oldOfferAuthority, true),
                bossBuy1TokenAccount: getAssociatedTokenAddressSync(offer.buyToken1.mint, BOSS, true),
                bossSellTokenAccount: getAssociatedTokenAddressSync(offer.sellTokenMint, BOSS, true),
                state: statePda,
                offerTokenAuthority: oldOfferAuthority,
                boss: BOSS,
            })
            .instruction();

        const tx = await program.methods
            .makeOfferOne(
              offerId,
              buyTokenAmount,
              sellTokenStartAmount,
              sellTokenEndAmount,
              new BN(offerStartTime),
              new BN(offerEndTime),
              priceFixDuration
            ).accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: getAssociatedTokenAddressSync(offer.sellTokenMint, offerAuthority, true),
                offerBuyToken1Account: getAssociatedTokenAddressSync(offer.buyToken1.mint, offerAuthority, true),
                offerTokenAuthority: offerAuthority,
                bossBuyToken1Account: getAssociatedTokenAddressSync(offer.buyToken1.mint, BOSS, true),
                sellTokenMint: offer.sellTokenMint,
                buyToken1Mint: offer.buyToken1.mint,
                state: statePda,
                boss: BOSS,
            })
            .preInstructions([closeInstruction, offerBuyTokenAccountInstruction, offerSellTokenAccountInstruction])
            .transaction();

        tx.feePayer = BOSS;
        tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

        const serializedTx = tx.serialize({
            requireAllSignatures: false,
            verifySignatures: false,
        });

        const base58Tx = bs58.encode(serializedTx);
        console.log('Replace Offer Transaction (Base58):');
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
        console.error('Failed to create replace offer transaction:', error);
    }
}

await main();
