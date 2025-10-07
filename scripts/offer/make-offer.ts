import { PublicKey } from '@solana/web3.js';
import { ScriptHelper } from '../utils/script-helper';

// Token addresses
const TOKEN_IN_MINT = new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'); // USDC
const TOKEN_OUT_MINT = new PublicKey('5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5'); // ONyc

// Test addresses
// const TOKEN_IN_MINT = new PublicKey('qaegW5BccnepuexbHkVqcqQUuEwgDMqCCo1wJ4fWeQu');  // TestUSDC
// const TOKEN_OUT_MINT = new PublicKey('5Uzafw84V9rCTmYULqdJA115K6zHP16vR15zrcqa6r6C');  // TestONe

async function createMakeOfferTransaction() {
    const helper = await ScriptHelper.create();

    console.log('Creating make offer transaction...');
    console.log('Token In (USDC):', TOKEN_IN_MINT.toBase58());
    console.log('Token Out (ONe):', TOKEN_OUT_MINT.toBase58());

    const boss = await helper.getBoss();
    console.log('Boss:', boss.toBase58());

    try {
        const tx = await helper.buildMakeOfferTransaction({
            tokenInMint: TOKEN_IN_MINT,
            tokenOutMint: TOKEN_OUT_MINT,
            feeBasisPoints: 0, // 0% fee
            needsApproval: false, // No approval required
            allowPermissionless: false // Standard flow only
        });

        return helper.printTransaction(tx, 'Make Offer Transaction');
    } catch (error) {
        console.error('Error creating transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createMakeOfferTransaction();
    } catch (error) {
        console.error('Failed to create make offer transaction:', error);
    }
}

await main();