import * as anchor from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';
import { BN } from 'bn.js';

import { getBossAccount, initProgram, PROGRAM_ID, RPC_URL } from '../utils/script-commons';

async function createCloseSingleRedemptionOfferTransaction() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(RPC_URL);

    // Configuration - Specify the offer ID to close
    const offerId = new BN(1); // Change this to the offer ID you want to close

    console.log('Closing single redemption offer with ID:', offerId.toString());

    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);

    const BOSS = await getBossAccount(program);
    console.log('BOSS:', BOSS.toBase58());

    // First, let's try to fetch the single redemption offers to verify the offer exists
    try {
        const [singleRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync(
            [Buffer.from('single_redemption_offers')],
            PROGRAM_ID
        );
        
        const singleRedemptionOfferAccount = await program.account.singleRedemptionOfferAccount.fetch(
            singleRedemptionOfferAccountPda
        );
        
        const targetOffer = singleRedemptionOfferAccount.offers.find(offer => 
            offer.offerId.eq(offerId)
        );
        
        if (!targetOffer) {
            throw new Error(`Offer with ID ${offerId.toString()} not found`);
        }
        
        console.log('Found offer to close:');
        console.log('- Token In Mint:', targetOffer.tokenInMint.toBase58());
        console.log('- Token Out Mint:', targetOffer.tokenOutMint.toBase58());
        console.log('- Price:', (targetOffer.price.toNumber() / 1e9).toFixed(9), 'output tokens per 1 input token');
        console.log('- Start Time:', new Date(targetOffer.startTime.toNumber() * 1000).toISOString());
        console.log('- End Time:', new Date(targetOffer.endTime.toNumber() * 1000).toISOString());
        
        // Check if offer is currently active
        const now = Math.floor(Date.now() / 1000);
        const isActive = now >= targetOffer.startTime.toNumber() && now <= targetOffer.endTime.toNumber();
        console.log('- Status:', isActive ? '🟢 ACTIVE' : '🔴 INACTIVE');
        
    } catch (error) {
        console.error('Error fetching single redemption offers:', error);
        console.log('Proceeding with close attempt anyway...');
    }

    try {
        const tx = await program.methods
            .closeSingleRedemptionOffer(offerId)
            .accountsPartial({
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
        console.log('Close Single Redemption Offer Transaction (Base58):');
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error('Error creating close single redemption offer transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createCloseSingleRedemptionOfferTransaction();
    } catch (error) {
        console.error('Failed to create close single redemption offer transaction:', error);
    }
}

await main();