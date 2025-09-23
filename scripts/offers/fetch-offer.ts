import { ScriptHelper } from '../utils/script-helper';
import { getMint } from '@solana/spl-token';

// Configuration
const OFFER_ID = 1;

// Helper function to format timestamp to human readable date
function formatTimestamp(timestamp: number): string {
    if (timestamp === 0) return 'Not set';
    return new Date(timestamp * 1000).toISOString();
}

async function fetchOffer() {
    const helper = await ScriptHelper.create();

    console.log('Fetching offer details...');
    console.log('Offer ID:', OFFER_ID);

    try {
        const offer = await helper.getOffer(OFFER_ID);
        if (!offer) {
            console.log(`Offer ${OFFER_ID} not found.`);
            return;
        }

        console.log('\n📋 OFFER DETAILS');
        console.log('================');
        console.log(`Offer ID: ${offer.offerId.toString()}`);
        console.log(`Token In:  ${offer.tokenInMint.toBase58()}`);
        console.log(`Token Out: ${offer.tokenOutMint.toBase58()}`);
        console.log(`Fee: ${(offer.feeBasisPoints.toNumber() / 100).toFixed(2)}%`);

        // Fetch token mint info for better display
        try {
            const tokenInMint = await getMint(helper.connection, offer.tokenInMint);
            const tokenOutMint = await getMint(helper.connection, offer.tokenOutMint);
            console.log(`Token In Decimals: ${tokenInMint.decimals}`);
            console.log(`Token Out Decimals: ${tokenOutMint.decimals}`);
        } catch (error) {
            console.log('Could not fetch mint info:', error.message);
        }

        // Show vectors
        const activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() > 0);
        console.log(`\nVectors: ${activeVectors.length} configured`);

        if (activeVectors.length > 0) {
            console.log('\n🔢 VECTOR DETAILS');
            console.log('=================');

            for (const vector of activeVectors) {
                console.log(`\nVector #${vector.vectorId.toString()}:`);
                console.log(`  Start Time: ${formatTimestamp(vector.startTime.toNumber())}`);
                console.log(`  Base Time:  ${formatTimestamp(vector.baseTime.toNumber())}`);
                console.log(`  Base Price: ${vector.basePrice.toString()}`);
                console.log(`  APR: ${(vector.apr.toNumber() / 1_000_000).toFixed(4)}%`);
                console.log(`  Price Fix Duration: ${vector.priceFixDuration.toNumber()}s`);

                // Check if vector is currently active
                const now = Math.floor(Date.now() / 1000);
                const isActive = now >= vector.startTime.toNumber();
                console.log(`  Status: ${isActive ? '🟢 ACTIVE' : '🔴 PENDING'}`);
            }
        } else {
            console.log('\n⚠️  No vectors configured for this offer');
        }

    } catch (error) {
        console.error('Error fetching offer:', error);
        if (error.message?.includes('Account does not exist')) {
            console.log('Offers account not initialized. Run initialize-offers script first.');
        }
    }
}

async function main() {
    try {
        await fetchOffer();
    } catch (error) {
        console.error('Failed to fetch offer:', error);
    }
}

await main();