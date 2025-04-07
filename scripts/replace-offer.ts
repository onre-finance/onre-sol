import * as anchor from '@coral-xyz/anchor';
import {
    createAssociatedTokenAccountInstruction,
    getAssociatedTokenAddressSync,
    TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import bs58 from 'bs58';
import { BN } from 'bn.js';

import { getBossAccount, getOffer, initProgram, PROGRAM_ID } from './script-commons';
import { PublicKey } from '@solana/web3.js';

async function createMakeOfferOneTransaction() {
    const oldOfferId = new BN(1);
    const offerId = new BN(1);

    const buyTokenAmount = new BN(150e9);
    const sellTokenAmount = new BN(150e9);

    const program = await initProgram();
    const connection = new anchor.web3.Connection(process.env.SOL_MAINNET_RPC_URL || '');

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
        getAssociatedTokenAddressSync(offer.buyTokenMint1, offerAuthority, true),
        offerAuthority,
        offer.buyTokenMint1,
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
                offerBuy1TokenAccount: getAssociatedTokenAddressSync(offer.buyTokenMint1, oldOfferAuthority, true),
                bossBuy1TokenAccount: getAssociatedTokenAddressSync(offer.buyTokenMint1, BOSS, true),
                bossSellTokenAccount: getAssociatedTokenAddressSync(offer.sellTokenMint, BOSS, true),
                state: statePda,
                offerTokenAuthority: oldOfferAuthority,
                boss: BOSS,
            })
            .instruction();

        const tx = await program.methods
            .makeOfferOne(offerId, buyTokenAmount, sellTokenAmount)
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: getAssociatedTokenAddressSync(offer.sellTokenMint, offerAuthority, true),
                offerBuyToken1Account: getAssociatedTokenAddressSync(offer.buyTokenMint1, offerAuthority, true),
                offerTokenAuthority: offerAuthority,
                bossBuyToken1Account: getAssociatedTokenAddressSync(offer.buyTokenMint1, BOSS, true),
                sellTokenMint: offer.sellTokenMint,
                buyToken1Mint: offer.buyTokenMint1,
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
