import { PublicKey } from '@solana/web3.js';
import { getMint } from '@solana/spl-token';
import { ScriptHelper } from '../utils/script-helper';

// Helper function to format timestamp to human readable date
function formatTimestamp(timestamp: number): string {
    if (timestamp === 0) return 'Not set';
    return new Date(timestamp * 1000).toISOString();
}

// Cache for token decimals to avoid repeated fetches
const tokenDecimalsCache: Map<string, number> = new Map();

// Helper function to get token decimals from chain
async function getTokenDecimals(helper: ScriptHelper, mintAddress: PublicKey): Promise<number> {
    const mintKey = mintAddress.toBase58();

    // Check cache first
    if (tokenDecimalsCache.has(mintKey)) {
        return tokenDecimalsCache.get(mintKey)!;
    }

    try {
        const mintInfo = await getMint(helper.connection, mintAddress);
        const decimals = mintInfo.decimals;
        tokenDecimalsCache.set(mintKey, decimals);
        return decimals;
    } catch (error) {
        console.warn(`Failed to fetch decimals for ${mintKey}, defaulting to 9:`, error);
        // Default to 9 decimals if we can't fetch mint info
        tokenDecimalsCache.set(mintKey, 9);
        return 9;
    }
}

// Helper function to format token amount with proper decimals
async function formatTokenAmount(helper: ScriptHelper, amount: number, mintAddress: PublicKey): Promise<string> {
    if (amount === 0) return '0';

    const decimals = await getTokenDecimals(helper, mintAddress);
    const formatted = (amount / Math.pow(10, decimals)).toFixed(Math.min(decimals, 9)); // Cap display at 9 decimal places
    return `${formatted} (${decimals} decimals)`;
}

// Helper function to format APR
function formatAPR(apr: number): string {
    return `${(apr / 1_000_000).toFixed(4)}%`;
}

async function getAllOffers() {
    const helper = await ScriptHelper.create();

    console.log('========================================');
    console.log('         ONRE OFFERS OVERVIEW');
    console.log('========================================\n');

    try {
        // Fetch the unified offer account
        console.log('🏪 OFFERS ACCOUNT');
        console.log('=================');

        const offerAccount = await helper.getOfferAccount();
        console.log(`Account PDA: ${helper.pdas.offerAccountPda.toBase58()}`);
        console.log(`Total offers created: ${offerAccount.counter.toString()}\n`);

        let activeOffers = 0;

        for (const offer of offerAccount.offers) {
            if (offer.offerId.toNumber() !== 0) {
                activeOffers++;
                console.log(`📋 Offer #${offer.offerId.toString()}`);
                console.log(`   Token In:  ${offer.tokenInMint.toBase58()}`);
                console.log(`   Token Out: ${offer.tokenOutMint.toBase58()}`);
                console.log(`   Fee: ${(offer.feeBasisPoints.toNumber() / 100).toFixed(2)}%`);
                console.log(`   Vectors: ${offer.vectors.filter(v => v.vectorId.toNumber() > 0).length} configured`);

                // Show active vectors
                for (const vector of offer.vectors) {
                    if (vector.vectorId.toNumber() !== 0) {
                        console.log(`   Vector #${vector.vectorId.toString()}:`);
                        console.log(`     Start Time: ${formatTimestamp(vector.startTime.toNumber())}`);
                        console.log(`     Base Price: ${await formatTokenAmount(helper, vector.basePrice.toNumber(), offer.tokenOutMint)}`);
                        console.log(`     APR: ${formatAPR(vector.apr.toNumber())}`);
                        console.log(`     Price Fix Duration: ${vector.priceFixDuration.toNumber()}s`);

                        // Check if vector is currently active
                        const now = Math.floor(Date.now() / 1000);
                        const isActive = now >= vector.startTime.toNumber();
                        console.log(`     Status: ${isActive ? '🟢 ACTIVE' : '🔴 PENDING'}`);
                    }
                }
                console.log('');
            }
        }

        if (activeOffers === 0) {
            console.log('   No active offers found.\n');
        } else {
            console.log(`Found ${activeOffers} active offers.\n`);
        }

    } catch (error) {
        console.error('Error fetching offers:', error);
        if (error.message?.includes('Account does not exist')) {
            console.log('Offers account not initialized. Run initialize-offers script first.');
        }
    }
}

async function main() {
    try {
        await getAllOffers();
    } catch (error) {
        console.error('Failed to fetch offers:', error);
    }
}

await main();