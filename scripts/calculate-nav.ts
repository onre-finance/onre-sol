// scripts/calculate-nav.ts
import { BN } from 'bn.js';
import { getOffers, initProgram } from './script-commons';
import { Program } from '@coral-xyz/anchor';
import { OnreApp } from '../target/types/onre_app';

const USDC_MINT = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

async function fetchOffers() {
    const program = await initProgram();

    try {
        const offerAccounts = await getOffers(program);

        let bestOffer: { price: number; validUntil: number } | null = null;

        for (const offerAccount of offerAccounts) {
            const offer = offerAccount.account;

            if (offer.sellTokenMint.toBase58() === USDC_MINT) {
                try {
                    const priceInfo = calculateOfferPrice(offerAccount);

                    if (bestOffer === null || priceInfo.price < bestOffer.price) {
                        bestOffer = priceInfo;
                    }
                } catch (error) {
                    console.warn('Skipping offer due to error:', error.message);
                }
            }
        }

        if (bestOffer === null) {
            return { price: 1.0, validUntil: Math.floor(Date.now() / 1000) + 3600 }; // Default 1 USDC per ONe
        }

        return bestOffer;
    } catch (error) {
        console.error('Error fetching offer:', error);
        throw error;
    }
}

function calculateOfferPrice(
    offerAccount: Awaited<ReturnType<Program<OnreApp>['account']['offer']['all']>>[0],
    buyTokenDecimals: number = 9,  // ONe token decimals
    sellTokenDecimals: number = 6, // USDC decimals
    now = new Date()
): { price: number; validUntil: number } {
    const offer = offerAccount.account;

    const { buyToken1, sellTokenStartAmount, sellTokenEndAmount, offerStartTime, offerEndTime, priceFixDuration } = offer;

    const offerStartUTime = offerStartTime.toNumber();
    const offerEndUTime = offerEndTime.toNumber();
    const priceFixDurationSec = priceFixDuration.toNumber();

    const currentUTime = Math.floor(now.getTime() / 1000);
    const totalIntervals = Math.floor((offerEndUTime - offerStartUTime) / priceFixDurationSec);
    const currentInterval = Math.floor((currentUTime - offerStartUTime) / priceFixDurationSec);
    const nextIntervalStartSec =
        offerStartUTime + Math.max(0, Math.min(totalIntervals, currentInterval + 1)) * priceFixDurationSec;

    const [progressDividend, progressDivisor] =
        currentInterval < 0 || totalIntervals < 1 ? [0, 1]
        : currentInterval + 1 > totalIntervals ? [1, 1]
        : [currentInterval + 1, totalIntervals];

    const currentSellAmount = sellTokenStartAmount.add(
        sellTokenEndAmount.sub(sellTokenStartAmount).mul(new BN(progressDividend)).div(new BN(progressDivisor)),
    );

    const decimalDiff = Math.abs(buyTokenDecimals - sellTokenDecimals);
    const scaleDiff = new BN(10).pow(new BN(decimalDiff));

    const [buyAmount, sellAmount] =
        buyTokenDecimals > sellTokenDecimals ?
            [buyToken1.amount.div(scaleDiff), currentSellAmount]
        : [buyToken1.amount, currentSellAmount.div(scaleDiff)];

    // Calculate price as sell tokens per buy token
    const price = parseInt(sellAmount.toString()) / parseInt(buyAmount.toString());
    return { price, validUntil: nextIntervalStartSec };
}

async function main() {
    try {
        const result = await fetchOffers();

        console.log('Current NAV (Net Asset Value):');
        console.log('Price per ONe token in USDC:', result.price);
        console.log('Price valid until:', new Date(result.validUntil * 1000).toISOString());
        console.log('Price valid for next:', Math.max(0, result.validUntil - Math.floor(Date.now() / 1000)), 'seconds');

        // Also show the raw calculation for debugging
        console.log('Raw result:', result);
    } catch (error) {
        console.error('Failed to calculate NAV:', error);
    }
}

await main();
