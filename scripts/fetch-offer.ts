// scripts/fetchOffer.ts
import { BN } from 'bn.js';
import { getOffer, initProgram } from './script-commons';

// @ts-ignore
async function fetchOffer(offerId: BN) {
    const program = await initProgram();

    try {
        const offerAccount = await getOffer(offerId, program);

        console.log('Offer Account Data:');
        console.log('  Offer ID:', offerAccount.offerId.toString());
        console.log('  Sell Token Mint:', offerAccount.sellTokenMint.toString());
        console.log('  Buy Token Mint 1:', offerAccount.buyToken1.mint.toString());
        console.log('  Buy Token Mint 2:', offerAccount.buyToken2.mint.toString());
        console.log('  Buy Token 1 Total Amount:', offerAccount.buyToken1.amount.toString());
        console.log('  Buy Token 2 Total Amount:', offerAccount.buyToken2.amount.toString());
        console.log('  Sell Token Start Amount:', offerAccount.sellTokenStartAmount.toString());
        console.log('  Sell Token End Amount:', offerAccount.sellTokenEndAmount.toString());
        console.log('  Offer Start Time:', new Date(offerAccount.offerStartTime.toNumber() * 1000));
        console.log('  Offer End Time:', new Date(offerAccount.offerEndTime.toNumber() * 1000));
        console.log('  Price Fix Duration:', offerAccount.priceFixDuration.toString());
        console.log('  Authority Bump:', offerAccount.authorityBump);

        return offerAccount;
    } catch (error) {
        console.error('Error fetching offer:', error);
        throw error;
    }
}

async function main() {
    try {
        const offerId = new BN(1); // Replace with the desired offer ID
        await fetchOffer(offerId);
    } catch (error) {
        console.error('Failed to fetch offer:', error);
    }
}

await main();
