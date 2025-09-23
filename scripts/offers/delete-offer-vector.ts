import { ScriptHelper } from '../utils/script-helper';

// Configuration
const OFFER_ID = 1;
const VECTOR_ID = 1;

async function createDeleteOfferVectorTransaction() {
    const helper = await ScriptHelper.create();

    console.log('Creating delete offer vector transaction...');
    console.log('Offer ID:', OFFER_ID);
    console.log('Vector ID:', VECTOR_ID);

    const boss = await helper.getBoss();
    console.log('Boss:', boss.toBase58());

    try {
        // Check if offer exists
        const offer = await helper.getOffer(OFFER_ID);
        if (!offer) {
            throw new Error(`Offer ${OFFER_ID} not found.`);
        }

        // Check if vector exists
        const vector = offer.vectors.find(v => v.vectorId.toNumber() === VECTOR_ID);
        if (!vector || vector.vectorId.toNumber() === 0) {
            throw new Error(`Vector ${VECTOR_ID} not found in offer ${OFFER_ID}.`);
        }

        console.log('Found offer:', {
            id: offer.offerId.toNumber(),
            tokenIn: offer.tokenInMint.toBase58(),
            tokenOut: offer.tokenOutMint.toBase58(),
            vectors: offer.vectors.filter(v => v.vectorId.toNumber() > 0).length
        });

        console.log('Found vector:', {
            id: vector.vectorId.toNumber(),
            startTime: new Date(vector.startTime.toNumber() * 1000).toISOString(),
            basePrice: vector.basePrice.toNumber(),
            apr: vector.apr.toNumber() / 1_000_000 + '%'
        });

        const tx = await helper.buildDeleteOfferVectorTransaction({
            offerId: OFFER_ID,
            vectorId: VECTOR_ID
        });

        return helper.printTransaction(tx, 'Delete Offer Vector Transaction');
    } catch (error) {
        console.error('Error creating transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createDeleteOfferVectorTransaction();
    } catch (error) {
        console.error('Failed to create delete offer vector transaction:', error);
    }
}

await main();