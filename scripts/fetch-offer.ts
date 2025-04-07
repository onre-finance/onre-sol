// scripts/fetchOffer.ts
import { BN } from 'bn.js';
import { getOffer, initProgram } from './script-commons';

async function fetchOffer(offerId: BN) {
    const program = await initProgram();

    try {
        const offerAccount = await getOffer(offerId, program);

        console.log('Offer Account Data:');
        console.log('  Offer ID:', offerAccount.offerId.toString());
        console.log('  Sell Token Mint:', offerAccount.sellTokenMint.toString());
        console.log('  Buy Token Mint 1:', offerAccount.buyTokenMint1.toString());
        console.log('  Buy Token Mint 2:', offerAccount.buyTokenMint2.toString());
        console.log('  Buy Token 1 Total Amount:', offerAccount.buyToken1TotalAmount.toString());
        console.log('  Buy Token 2 Total Amount:', offerAccount.buyToken2TotalAmount.toString());
        console.log('  Sell Token Total Amount:', offerAccount.sellTokenTotalAmount.toString());
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
