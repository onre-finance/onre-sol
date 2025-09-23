import { ScriptHelper } from '../utils/script-helper';

// Configuration
const OFFER_ID = 1;
const NEW_FEE_BASIS_POINTS = 250; // 2.5% fee

async function createUpdateOfferFeeTransaction() {
    const helper = await ScriptHelper.create();

    console.log('Creating update offer fee transaction...');
    console.log('Offer ID:', OFFER_ID);
    console.log('New Fee:', NEW_FEE_BASIS_POINTS / 100, '%');

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
            currentFee: offer.feeBasisPoints.toNumber() / 100 + '%',
            newFee: NEW_FEE_BASIS_POINTS / 100 + '%'
        });

        const tx = await helper.buildUpdateOfferFeeTransaction({
            offerId: OFFER_ID,
            newFee: NEW_FEE_BASIS_POINTS
        });

        return helper.printTransaction(tx, 'Update Offer Fee Transaction');
    } catch (error) {
        console.error('Error creating transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createUpdateOfferFeeTransaction();
    } catch (error) {
        console.error('Failed to create update offer fee transaction:', error);
    }
}

await main();