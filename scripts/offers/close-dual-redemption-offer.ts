import * as anchor from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';
import { BN } from 'bn.js';

import { getBossAccount, initProgram, PROGRAM_ID, RPC_URL } from './script-commons';

async function createCloseDualRedemptionOfferTransaction() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(RPC_URL);

    // Configuration - Specify the offer ID to close
    const offerId = new BN(1); // Change this to the offer ID you want to close

    console.log('Closing dual redemption offer with ID:', offerId.toString());

    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], PROGRAM_ID);

    const BOSS = await getBossAccount(program);
    console.log('BOSS:', BOSS.toBase58());

    // First, let's try to fetch the dual redemption offers to verify the offer exists
    try {
        const [dualRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync(
            [Buffer.from('dual_redemption_offers')],
            PROGRAM_ID
        );
        
        const dualRedemptionOfferAccount = await program.account.dualRedemptionOfferAccount.fetch(
            dualRedemptionOfferAccountPda
        );
        
        const targetOffer = dualRedemptionOfferAccount.offers.find(offer => 
            offer.offerId.eq(offerId)
        );
        
        if (!targetOffer) {
            throw new Error(`Offer with ID ${offerId.toString()} not found`);
        }
        
        console.log('Found offer to close:');
        console.log('- Token In Mint:', targetOffer.tokenInMint.toBase58());
        console.log('- Token Out Mint 1:', targetOffer.tokenOutMint1.toBase58());
        console.log('- Token Out Mint 2:', targetOffer.tokenOutMint2.toBase58());
        console.log('- Price 1:', targetOffer.price1.toString());
        console.log('- Price 2:', targetOffer.price2.toString());
        console.log('- Ratio Basis Points:', targetOffer.ratioBasisPoints.toString());
        console.log('- Start Time:', new Date(targetOffer.startTime.toNumber() * 1000).toISOString());
        console.log('- End Time:', new Date(targetOffer.endTime.toNumber() * 1000).toISOString());
        
    } catch (error) {
        console.error('Error fetching dual redemption offers:', error);
        console.log('Proceeding with close attempt anyway...');
    }

    try {
        const tx = await program.methods
            .closeDualRedemptionOffer(offerId)
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
        console.log('Close Dual Redemption Offer Transaction (Base58):');
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error('Error creating close dual redemption offer transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createCloseDualRedemptionOfferTransaction();
    } catch (error) {
        console.error('Failed to create close dual redemption offer transaction:', error);
    }
}

await main();