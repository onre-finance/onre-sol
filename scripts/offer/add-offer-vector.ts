import { PublicKey } from '@solana/web3.js';
import { ScriptHelper } from '../utils/script-helper';

// Token addresses
const TOKEN_IN_MINT = new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'); // USDC
const TOKEN_OUT_MINT = new PublicKey('5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5'); // ONyc

// Configuration for the offer vector
const BASE_TIME = Math.floor(new Date(Date.UTC(2025, 4, 27, 0, 0, 0)).getTime() / 1000); // May 27, 2025
const BASE_PRICE = 1_000_000_000; // 1.0 (scaled by 1,000,000,000) all prices are scaled by 9 decimals
const APR = 36_500; // 0.0365% APR (scaled by 1,000,000)
const PRICE_FIX_DURATION = 60 * 60 * 24; // 1 day

async function createAddOfferVectorTransaction() {
    const helper = await ScriptHelper.create();

    console.log('Creating add offer vector transaction...');
    console.log('Token In (USDC):', TOKEN_IN_MINT.toBase58());
    console.log('Token Out (ONe):', TOKEN_OUT_MINT.toBase58());
    console.log('Base Time:', new Date(BASE_TIME * 1000).toISOString());
    console.log('Base Price:', BASE_PRICE);
    console.log('APR:', APR / 1_000_000, '%');
    console.log('Price Fix Duration:', PRICE_FIX_DURATION, 'seconds');

    const boss = await helper.getBoss();
    console.log('Boss:', boss.toBase58());

    try {
        // Check if offer exists
        const offer = await helper.getOffer(TOKEN_IN_MINT, TOKEN_OUT_MINT);
        if (!offer) {
            throw new Error(`Offer for ${TOKEN_IN_MINT.toBase58()} -> ${TOKEN_OUT_MINT.toBase58()} not found. Create the offer first using make-offer script.`);
        }

        console.log('Found offer:', {
            tokenIn: offer.tokenInMint.toBase58(),
            tokenOut: offer.tokenOutMint.toBase58(),
            feeBasisPoints: offer.feeBasisPoints,
            needsApproval: offer.needsApproval != 0,
            allowPermissionless: offer.allowPermissionless != 0,
            vectors: offer.vectors.filter(v => v.startTime.toNumber() > 0).length
        });

        const tx = await helper.buildAddOfferVectorTransaction({
            tokenInMint: TOKEN_IN_MINT,
            tokenOutMint: TOKEN_OUT_MINT,
            baseTime: BASE_TIME,
            basePrice: BASE_PRICE,
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