import { PublicKey } from '@solana/web3.js';
import { ScriptHelper } from '../utils/script-helper';

// Token addresses
const TOKEN_IN_MINT = new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'); // USDC
const TOKEN_OUT_MINT = new PublicKey('5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5'); // ONyc

// Configuration
const NEW_FEE_BASIS_POINTS = 250; // 2.5% fee

async function createUpdateOfferFeeTransaction() {
    const helper = await ScriptHelper.create();

    console.log('Creating update offer fee transaction...');
    console.log('Token In (USDC):', TOKEN_IN_MINT.toBase58());
    console.log('Token Out (ONe):', TOKEN_OUT_MINT.toBase58());
    console.log('New Fee:', NEW_FEE_BASIS_POINTS / 100, '%');

    const boss = await helper.getBoss();
    console.log('Boss:', boss.toBase58());

    try {
        // Check if offer exists
        const offer = await helper.getOffer(TOKEN_IN_MINT, TOKEN_OUT_MINT);
        if (!offer) {
            throw new Error(`Offer for ${TOKEN_IN_MINT.toBase58()} -> ${TOKEN_OUT_MINT.toBase58()} not found.`);
        }

        console.log('Found offer:', {
            tokenIn: offer.tokenInMint.toBase58(),
            tokenOut: offer.tokenOutMint.toBase58(),
            currentFee: offer.feeBasisPoints / 100 + '%',
            newFee: NEW_FEE_BASIS_POINTS / 100 + '%',
            needsApproval: offer.needsApproval != 0,
            allowPermissionless: offer.allowPermissionless != 0,
        });

        const tx = await helper.buildUpdateOfferFeeTransaction({
            tokenInMint: TOKEN_IN_MINT,
            tokenOutMint: TOKEN_OUT_MINT,
            newFeeBasisPoints: NEW_FEE_BASIS_POINTS
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