import { ScriptHelper } from '../utils/script-helper';

// Configuration
const OFFER_ID = 1;

async function createCloseOfferTransaction() {
    const helper = await ScriptHelper.create();

    console.log('Creating close offer transaction...');
    console.log('Offer ID:', OFFER_ID);

    const boss = await helper.getBoss();
    console.log('Boss:', boss.toBase58());

    try {
        // Check if offer exists
        const offer = await helper.getOffer(OFFER_ID);
        if (!offer) {
            throw new Error(`Offer ${OFFER_ID} not found.`);
        }

        console.log('Found offer:', {
            id: offer.offerId.toNumber(),
            tokenIn: offer.tokenInMint.toBase58(),
            tokenOut: offer.tokenOutMint.toBase58(),
            vectors: offer.vectors.filter(v => v.vectorId.toNumber() > 0).length
        });

        const tx = await helper.buildCloseOfferTransaction({
            offerId: OFFER_ID
        });

        return helper.printTransaction(tx, 'Close Offer Transaction');
    } catch (error) {
        console.error('Error creating transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createCloseOfferTransaction();
    } catch (error) {
        console.error('Failed to create close offer transaction:', error);
    }
}

await main();