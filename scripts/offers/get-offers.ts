import * as anchor from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import { getMint } from '@solana/spl-token';
import { initProgram, PROGRAM_ID, RPC_URL } from '../utils/script-commons';

// Helper function to format timestamp to human readable date
function formatTimestamp(timestamp: number): string {
    if (timestamp === 0) return 'Not set';
    return new Date(timestamp * 1000).toISOString();
}

// Cache for token decimals to avoid repeated fetches
const tokenDecimalsCache: Map<string, number> = new Map();

// Helper function to get token decimals from chain
async function getTokenDecimals(connection: anchor.web3.Connection, mintAddress: PublicKey): Promise<number> {
    const mintKey = mintAddress.toBase58();
    
    // Check cache first
    if (tokenDecimalsCache.has(mintKey)) {
        return tokenDecimalsCache.get(mintKey)!;
    }
    
    try {
        const mintInfo = await getMint(connection, mintAddress);
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
async function formatTokenAmount(connection: anchor.web3.Connection, amount: number, mintAddress: PublicKey): Promise<string> {
    if (amount === 0) return '0';
    
    const decimals = await getTokenDecimals(connection, mintAddress);
    const formatted = (amount / Math.pow(10, decimals)).toFixed(Math.min(decimals, 9)); // Cap display at 9 decimal places
    return `${formatted} (${decimals} decimals)`;
}

// Helper function to format ratio basis points as percentage
function formatRatioBasisPoints(basisPoints: number): string {
    return `${(basisPoints / 100).toFixed(2)}%`;
}

async function getAllOffers() {
    const program = await initProgram();
    const connection = new anchor.web3.Connection(RPC_URL);
    
    console.log('========================================');
    console.log('         ONRE OFFERS OVERVIEW');
    console.log('========================================\n');

    // Derive PDAs for the three offer account types
    const [buyOffersPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('buy_offers')], 
        PROGRAM_ID
    );
    
    const [singleRedemptionOffersPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('single_redemption_offers')], 
        PROGRAM_ID
    );
    
    const [dualRedemptionOffersPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('dual_redemption_offers')], 
        PROGRAM_ID
    );

    try {
        // Fetch Buy Offers
        console.log('🛒 BUY OFFERS');
        console.log('==============');
        try {
            const buyOffersAccount = await program.account.buyOfferAccount.fetch(buyOffersPda);
            console.log(`Account PDA: ${buyOffersPda.toBase58()}`);
            console.log(`Total offers created: ${buyOffersAccount.counter.toString()}\n`);
            
            let activeBuyOffers = 0;
            
            for (const offer of buyOffersAccount.offers) {
                if (offer.offerId.toNumber() !== 0) {
                    activeBuyOffers++;
                    console.log(`📋 Buy Offer #${offer.offerId.toString()}`);
                    console.log(`   Token In:  ${offer.tokenInMint.toBase58()}`);
                    console.log(`   Token Out: ${offer.tokenOutMint.toBase58()}`);
                    console.log(`   Time Segments: ${offer.timeSegments.length} configured`);
                    
                    // Show active time segments
                    for (const segment of offer.timeSegments) {
                        if (segment.segmentId.toNumber() !== 0) {
                            console.log(`   Segment ${segment.segmentId.toString()}:`);
                            console.log(`     Start: ${formatTimestamp(segment.startTime.toNumber())}`);
                            console.log(`     End:   ${formatTimestamp(segment.endTime.toNumber())}`);
                            console.log(`     Start Price: ${await formatTokenAmount(connection, segment.startPrice.toNumber(), offer.tokenOutMint)}`);
                            console.log(`     End Price:   ${await formatTokenAmount(connection, segment.endPrice.toNumber(), offer.tokenOutMint)}`);
                            console.log(`     Price Fix Duration: ${segment.priceFixDuration.toNumber()}s`);
                        }
                    }
                    console.log('');
                }
            }
            
            if (activeBuyOffers === 0) {
                console.log('   No active buy offers found.\n');
            }
        } catch (error) {
            console.log(`   Account not found or error: ${error}\n`);
        }

        // Fetch Single Redemption Offers
        console.log('🔄 SINGLE REDEMPTION OFFERS');
        console.log('===========================');
        try {
            const singleRedemptionAccount = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOffersPda);
            console.log(`Account PDA: ${singleRedemptionOffersPda.toBase58()}`);
            console.log(`Total offers created: ${singleRedemptionAccount.counter.toString()}\n`);
            
            let activeSingleOffers = 0;
            
            for (const offer of singleRedemptionAccount.offers) {
                if (offer.offerId.toNumber() !== 0) {
                    activeSingleOffers++;
                    console.log(`📋 Single Redemption Offer #${offer.offerId.toString()}`);
                    console.log(`   Token In:  ${offer.tokenInMint.toBase58()}`);
                    console.log(`   Token Out: ${offer.tokenOutMint.toBase58()}`);
                    console.log(`   Start Time: ${formatTimestamp(offer.startTime.toNumber())}`);
                    console.log(`   End Time:   ${formatTimestamp(offer.endTime.toNumber())}`);
                    console.log(`   Price: ${await formatTokenAmount(connection, offer.price.toNumber(), offer.tokenOutMint)} tokens out per token in`);
                    
                    // Check if offer is currently active
                    const now = Math.floor(Date.now() / 1000);
                    const isActive = now >= offer.startTime.toNumber() && now <= offer.endTime.toNumber();
                    console.log(`   Status: ${isActive ? '🟢 ACTIVE' : '🔴 INACTIVE'}`);
                    console.log('');
                }
            }
            
            if (activeSingleOffers === 0) {
                console.log('   No active single redemption offers found.\n');
            }
        } catch (error) {
            console.log(`   Account not found or error: ${error}\n`);
        }

        // Fetch Dual Redemption Offers
        console.log('🔄🔄 DUAL REDEMPTION OFFERS');
        console.log('===========================');
        try {
            const dualRedemptionAccount = await program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOffersPda);
            console.log(`Account PDA: ${dualRedemptionOffersPda.toBase58()}`);
            console.log(`Total offers created: ${dualRedemptionAccount.counter.toString()}\n`);
            
            let activeDualOffers = 0;
            
            for (const offer of dualRedemptionAccount.offers) {
                if (offer.offerId.toNumber() !== 0) {
                    activeDualOffers++;
                    console.log(`📋 Dual Redemption Offer #${offer.offerId.toString()}`);
                    console.log(`   Token In:    ${offer.tokenInMint.toBase58()}`);
                    console.log(`   Token Out 1: ${offer.tokenOutMint1.toBase58()}`);
                    console.log(`   Token Out 2: ${offer.tokenOutMint2.toBase58()}`);
                    console.log(`   Start Time: ${formatTimestamp(offer.startTime.toNumber())}`);
                    console.log(`   End Time:   ${formatTimestamp(offer.endTime.toNumber())}`);
                    console.log(`   Price 1: ${await formatTokenAmount(connection, offer.price1.toNumber(), offer.tokenOutMint1)} tokens out per token in`);
                    console.log(`   Price 2: ${await formatTokenAmount(connection, offer.price2.toNumber(), offer.tokenOutMint2)} tokens out per token in`);
                    console.log(`   Ratio: ${formatRatioBasisPoints(offer.ratioBasisPoints.toNumber())} to Token 1, ${formatRatioBasisPoints(10000 - offer.ratioBasisPoints.toNumber())} to Token 2`);
                    
                    // Check if offer is currently active
                    const now = Math.floor(Date.now() / 1000);
                    const isActive = now >= offer.startTime.toNumber() && now <= offer.endTime.toNumber();
                    console.log(`   Status: ${isActive ? '🟢 ACTIVE' : '🔴 INACTIVE'}`);
                    console.log('');
                }
            }
            
            if (activeDualOffers === 0) {
                console.log('   No active dual redemption offers found.\n');
            }
        } catch (error) {
            console.log(`   Account not found or error: ${error}\n`);
        }

    } catch (error) {
        console.error('Error fetching offers:', error);
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