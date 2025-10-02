import { PublicKey } from '@solana/web3.js';
import { ScriptHelper } from '../utils/script-helper';

// Token addresses - update the offer by closing and creating a new one
const TOKEN_IN_MINT = new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'); // USDC
const TOKEN_OUT_MINT = new PublicKey('5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5'); // ONyc

// Configuration
const NEW_FEE_BASIS_POINTS = 100; // 1% fee for the new offer
const NEW_NEEDS_APPROVAL = false;
const NEW_ALLOW_PERMISSIONLESS = true;

async function createReplaceOfferTransaction() {
    const helper = await ScriptHelper.create();

    console.log('Creating replace offer transaction...');
    console.log('This will close and recreate offer for token pair');
    console.log('Token In (USDC):', TOKEN_IN_MINT.toBase58());
    console.log('Token Out (ONe):', TOKEN_OUT_MINT.toBase58());
    console.log('New Fee:', NEW_FEE_BASIS_POINTS / 100, '%');
    console.log('New Needs Approval:', NEW_NEEDS_APPROVAL);
    console.log('New Allow Permissionless:', NEW_ALLOW_PERMISSIONLESS);

    const boss = await helper.getBoss();
    console.log('Boss:', boss.toBase58());

    try {
        // Check if the old offer exists
        const oldOffer = await helper.getOffer(TOKEN_IN_MINT, TOKEN_OUT_MINT);
        if (!oldOffer) {
            throw new Error(`Offer for ${TOKEN_IN_MINT.toBase58()} -> ${TOKEN_OUT_MINT.toBase58()} not found.`);
        }

        console.log('Found old offer:', {
            tokenIn: oldOffer.tokenInMint.toBase58(),
            tokenOut: oldOffer.tokenOutMint.toBase58(),
            currentFee: oldOffer.feeBasisPoints / 100 + '%',
            needsApproval: oldOffer.needsApproval != 0,
            allowPermissionless: oldOffer.allowPermissionless != 0
        });

        // First, close the old offer
        console.log('\nStep 1: Closing old offer...');
        const closeTx = await helper.buildCloseOfferTransaction({
            tokenInMint: TOKEN_IN_MINT,
            tokenOutMint: TOKEN_OUT_MINT
        });
        const closeBase58 = helper.printTransaction(closeTx, 'Close Old Offer Transaction');

        // Then, create a new offer
        console.log('\nStep 2: Creating new offer...');
        const makeTx = await helper.buildMakeOfferTransaction({
            tokenInMint: TOKEN_IN_MINT,
            tokenOutMint: TOKEN_OUT_MINT,
            feeBasisPoints: NEW_FEE_BASIS_POINTS,
            needsApproval: NEW_NEEDS_APPROVAL,
            allowPermissionless: NEW_ALLOW_PERMISSIONLESS
        });
        const makeBase58 = helper.printTransaction(makeTx, 'Make New Offer Transaction');

        console.log('\n=== INSTRUCTIONS ===');
        console.log('1. Execute the close transaction first');
        console.log('2. Wait for confirmation');
        console.log('3. Execute the make offer transaction');
        console.log('4. Add vectors to the new offer using add-offer-vector script');

        return { closeBase58, makeBase58 };
    } catch (error) {
        console.error('Error creating transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createReplaceOfferTransaction();
    } catch (error) {
        console.error('Failed to create replace offer transaction:', error);
    }
}

await main();