// scripts/fetchOffer.ts
import * as anchor from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import { BN } from 'bn.js';

import { getAssociatedTokenAddressSync } from '@solana/spl-token';
import bs58 from 'bs58';
import { getBossAccount, getOffer, initProgram, PROGRAM_ID } from './script-commons';

async function closeOffer() {
    const offerId = new BN(1);
    const connection = new anchor.web3.Connection(process.env.SOL_MAINNET_RPC_URL || '');

    const program = await initProgram();
    const BOSS = await getBossAccount(program);

    const offer = await getOffer(offerId, program);

    const [offerAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)],
        program.programId,
    );

    // Derive the state PDA
    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);

    const [offerPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)],
        PROGRAM_ID,
    );

    try {
        let tx = await program.methods
            .closeOfferOne()
            .accountsPartial({
                offer: offerPda,
                offerSellTokenAccount: getAssociatedTokenAddressSync(offer.sellTokenMint, offerAuthority, true),
                offerBuy1TokenAccount: getAssociatedTokenAddressSync(offer.buyTokenMint1, offerAuthority, true),
                bossBuy1TokenAccount: getAssociatedTokenAddressSync(offer.buyTokenMint1, BOSS, true),
                bossSellTokenAccount: getAssociatedTokenAddressSync(offer.sellTokenMint, BOSS, true),
                state: statePda,
                offerTokenAuthority: offerAuthority,
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

        console.log('Close Offer Transaction (Base58):');
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error('Error closing offer:', error);
        throw error;
    }
}

async function main() {
    try {
        await closeOffer();
    } catch (error) {
        console.error('Failed to close offer:', error);
    }
}

await main();
