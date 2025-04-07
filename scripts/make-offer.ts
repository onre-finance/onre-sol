import * as anchor from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import {
    createAssociatedTokenAccountInstruction,
    getAssociatedTokenAddressSync,
    TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import bs58 from 'bs58';
import { BN } from 'bn.js';

import { getBossAccount, initProgram, PROGRAM_ID } from './script-commons';

const SELL_TOKEN_MINT = new PublicKey('qaegW5BccnepuexbHkVqcqQUuEwgDMqCCo1wJ4fWeQu');
const BUY_TOKEN_MINT = new PublicKey('5Uzafw84V9rCTmYULqdJA115K6zHP16vR15zrcqa6r6C');

async function createMakeOfferOneTransaction() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(process.env.SOL_MAINNET_RPC_URL || '');

    const offerId = new BN(4);
    const buyTokenAmount = new BN(100e9);
    const sellTokenAmount = new BN(100e9);

    const [offerAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)],
        program.programId,
    );

    const BOSS = await getBossAccount(program);

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
    );
    // Derive the state PDA
    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);

    const [offerPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)],
        PROGRAM_ID,
    );
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
                boss: BOSS,
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

await main();
