import { PublicKey } from '@solana/web3.js';
import { ScriptHelper } from '../utils/script-helper';

// Token addresses
const TOKEN_IN_MINT = new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'); // USDC
const TOKEN_OUT_MINT = new PublicKey('5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5'); // ONyc

// Configuration
const VECTOR_START_TIMESTAMP = new Date(2025, 4, 27).getTime() / 1000; // May 27, 2025

async function createDeleteOfferVectorTransaction() {
    const helper = await ScriptHelper.create();

    console.log('Creating delete offer vector transaction...');
    console.log('Token In (USDC):', TOKEN_IN_MINT.toBase58());
    console.log('Token Out (ONe):', TOKEN_OUT_MINT.toBase58());
    console.log('Vector ID:', VECTOR_START_TIMESTAMP);

    const boss = await helper.getBoss();
    console.log('Boss:', boss.toBase58());

    try {
        // Check if offer exists
        const offer = await helper.getOffer(TOKEN_IN_MINT, TOKEN_OUT_MINT);
        if (!offer) {
            throw new Error(`Offer for ${TOKEN_IN_MINT.toBase58()} -> ${TOKEN_OUT_MINT.toBase58()} not found.`);
        }

        // Check if vector exists by finding it with matching startTime
        const vector = offer.vectors.find(v => v.startTime.toNumber() === VECTOR_START_TIMESTAMP);
        if (!vector) {
            throw new Error(`Vector with startTime ${VECTOR_START_TIMESTAMP} not found in offer ${TOKEN_IN_MINT.toBase58()} -> ${TOKEN_OUT_MINT.toBase58()}.`);
        }

        console.log('Found offer:', {
            tokenIn: offer.tokenInMint.toBase58(),
            tokenOut: offer.tokenOutMint.toBase58(),
            feeBasisPoints: offer.feeBasisPoints,
            needsApproval: offer.needsApproval != 0,
            allowPermissionless: offer.allowPermissionless != 0,
            vectors: offer.vectors.filter(v => v.startTime.toNumber() > 0).length
        });

        console.log('Found vector:', {
            index: VECTOR_START_TIMESTAMP,
            startTime: new Date(vector.startTime.toNumber() * 1000).toISOString(),
            baseTime: new Date(vector.baseTime.toNumber() * 1000).toISOString(),
            basePrice: vector.basePrice.toNumber(),
            apr: vector.apr.toNumber() / 1_000_000 + '%',
            priceFixDuration: vector.priceFixDuration.toNumber()
        });

        const tx = await helper.buildDeleteOfferVectorTransaction({
            tokenInMint: TOKEN_IN_MINT,
            tokenOutMint: TOKEN_OUT_MINT,
            vectorStartTimestamp: VECTOR_START_TIMESTAMP
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