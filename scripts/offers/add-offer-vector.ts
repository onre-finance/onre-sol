import { ScriptHelper } from '../utils/script-helper';

// Configuration for the offer vector
const OFFER_ID = 1;
const START_TIME = Math.floor(new Date(Date.UTC(2025, 4, 27, 0, 0, 0)).getTime() / 1000); // May 27, 2025
const START_PRICE = 20160514420000; // 6 decimals for USDC (starting price)
const APR = 36_500; // 0.0365% APR (scaled by 1,000,000)
const PRICE_FIX_DURATION = 60 * 60 * 24; // 1 day

async function createAddOfferVectorTransaction() {
    const helper = await ScriptHelper.create();

    console.log('Creating add offer vector transaction...');
    console.log('Offer ID:', OFFER_ID);
    console.log('Start Time:', new Date(START_TIME * 1000).toISOString());
    console.log('Start Price:', START_PRICE);
    console.log('APR:', APR / 1_000_000, '%');
    console.log('Price Fix Duration:', PRICE_FIX_DURATION, 'seconds');

    const boss = await helper.getBoss();
    console.log('Boss:', boss.toBase58());

    try {
        // Check if offer exists
        const offer = await helper.getOffer(OFFER_ID);
        if (!offer) {
            throw new Error(`Offer ${OFFER_ID} not found. Create the offer first using make-offer script.`);
        }

        console.log('Found offer:', {
            id: offer.offerId.toNumber(),
            tokenIn: offer.tokenInMint.toBase58(),
            tokenOut: offer.tokenOutMint.toBase58(),
            vectors: offer.vectors.filter(v => v.vectorId.toNumber() > 0).length
        });

        const tx = await helper.buildAddOfferVectorTransaction({
            offerId: OFFER_ID,
            startTime: START_TIME,
            startPrice: START_PRICE,
            apr: APR,
            priceFixDuration: PRICE_FIX_DURATION
        });

        return helper.printTransaction(tx, 'Add Offer Vector Transaction');
    } catch (error) {
        console.error('Error creating transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createAddOfferVectorTransaction();
    } catch (error) {
        console.error('Failed to create add offer vector transaction:', error);
    }
}

await main();