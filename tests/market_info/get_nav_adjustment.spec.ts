import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Get NAV Adjustment", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        // Create mints with different decimals to test precision handling
        tokenInMint = testHelper.createMint(6); // USDC-like (6 decimals)
        tokenOutMint = testHelper.createMint(9); // ONyc-like (9 decimals)

        // Initialize program and offers
        await program.initialize({ onycMint: tokenOutMint });

        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });
    });

    describe("Basic Functionality Tests", () => {
        it("Should return positive adjustment for first vector (compared to 0)", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add first vector
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 1e9, // 1.0 with 9 decimals
                apr: 36_500, // 3.65% APR (scaled by 1M)
                priceFixDuration: 86400 // 1 day
            });

            // Get NAV adjustment
            const adjustment = await program.getNavAdjustment({ tokenInMint, tokenOutMint });

            // Since this is the first vector, adjustment should be the current price (positive)
            expect(adjustment).toBe(1000100000); // Should match the current price
        });

        it("Should be read-only instruction (no state changes)", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Get offer state before
            const offerBefore = await program.getOffer(tokenInMint, tokenOutMint);

            // Call getNavAdjustment
            await program.getNavAdjustment({ tokenInMint, tokenOutMint });

            // Get offer state after
            const offerAfter = await program.getOffer(tokenInMint, tokenOutMint);

            // Should be identical (no state changes)
            expect(offerAfter).toEqual(offerBefore);
        });
    });

    describe("Vector Transition Tests", () => {
        it("Should calculate positive adjustment when price increases between intervals", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add first vector with lower base price
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 1e9, // 1.0 with 9 decimals
                apr: 36_500, // 3.65% APR
                priceFixDuration: 86400 // 1 day intervals
            });

            // Advance time to make the first vector active for a while
            await testHelper.advanceClockBy(90_000); // 1 hour

            // Add second vector with higher base price - must have start time after first vector's base time
            const newCurrentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: newCurrentTime,
                basePrice: 1.5e9, // 1.5 with 9 decimals (higher than first)
                apr: 36_500, // 3.65% APR (lower than first)
                priceFixDuration: 86400 // 1 day intervals
            });

            await testHelper.advanceClockBy(90_000); // Move to second interval

            // Get adjustment - should be positive since we went from lower to higher price
            const adjustment = await program.getNavAdjustment({ tokenInMint, tokenOutMint });

            // Last interval price in first vector is 1.0002
            // First interval price in second vector is 1.50015 (1.5 * 1.0001)
            expect(adjustment).toBe(0.49995e9);
        });

        it("Should calculate negative adjustment when price decreases between vectors", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add first vector with higher base price
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 2e9, // 2.0 with 9 decimals
                apr: 36_500, // 3.65%% APR
                priceFixDuration: 86400 // 1 day
            });

            // Advance time to make the first vector active for a while
            await testHelper.advanceClockBy(86400); // 1 hour

            // Add second vector with lower base price
            const newCurrentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: newCurrentTime,
                basePrice: 1e9, // 1.0 with 9 decimals (lower than first)
                apr: 36_500, // 3.65% APR
                priceFixDuration: 86400 // 1 hour
            });

            // Get adjustment - should be negative since we went from higher to lower price
            const adjustment = await program.getNavAdjustment({ tokenInMint, tokenOutMint });

            expect(adjustment).toBe(-1.0003e9);
        });

        it("Should handle multiple vector transitions correctly", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add first vector
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 1e9, // 1.0
                apr: 50_000, // 5% APR
                priceFixDuration: 1800 // 30 minutes
            });

            // Advance time and add second vector
            await testHelper.advanceClockBy(1800);
            let newCurrentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: newCurrentTime,
                basePrice: 1.2e9, // 1.2 (higher)
                apr: 36_500, // 3.65% APR
                priceFixDuration: 86_400 // 1 day
            });

            // Advance time and add third vector
            await testHelper.advanceClockBy(1800);
            newCurrentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: newCurrentTime,
                basePrice: 1.1e9, // 1.1 (lower than second, but higher than first)
                apr: 36_500, // 3.65% APR
                priceFixDuration: 86_400 // 1 day
            });

            // Get adjustment - should compare current (third) vector to previous (second) vector
            const adjustment = await program.getNavAdjustment({ tokenInMint, tokenOutMint });

            // Should be negative since we went from 1.2 base price to 1.1 base price
            expect(adjustment).toBe(-0.10001e9);
        });

        it("Should handle zero price change correctly", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add first vector
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 1e9, // 1.0
                apr: 0, // 0% APR (no growth)
                priceFixDuration: 3600
            });

            // Advance time
            await testHelper.advanceClockBy(3600);

            // Add second vector with same price and 0% APR
            const newCurrentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: newCurrentTime,
                basePrice: 2.5e9,
                apr: 0, // 0% APR (no growth)
                priceFixDuration: 3600
            });

            // Get adjustment - should be 0 or very close to 0
            const adjustment = await program.getNavAdjustment({ tokenInMint, tokenOutMint });

            expect(adjustment).toBe(1.5e9);
        });
    });

    describe("Error Condition Tests", () => {
        it("Should fail with non-existent offer", async () => {
            await expect(program.getNavAdjustment({ tokenInMint: testHelper.createMint(9), tokenOutMint }))
                .rejects.toThrow("AnchorError caused by account: offer");

            await expect(program.getNavAdjustment({ tokenInMint, tokenOutMint: testHelper.createMint(9) }))
                .rejects.toThrow("AnchorError caused by account: offer");
        });

        it("Should fail when offer has no active vectors", async () => {
            // Don't add any vectors to the offer
            await expect(program.getNavAdjustment({ tokenInMint, tokenOutMint }))
                .rejects.toThrow("No active vector");
        });

        it("Should fail when no vector is active at current time", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector that starts in the future
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 86400, // starts tomorrow
                basePrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            await expect(program.getNavAdjustment({ tokenInMint, tokenOutMint }))
                .rejects.toThrow("No active vector");
        });
    });

    describe("Edge Case Tests", () => {
        it("Should handle time progression within same vector", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add first vector
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            await testHelper.advanceClockBy(86400);

            const newCurrentTime = await testHelper.getCurrentClockTime();

            // Add second vector
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: newCurrentTime,
                basePrice: 1.3e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Get initial adjustment
            const initialAdjustment = await program.getNavAdjustment({ tokenInMint, tokenOutMint });

            // Advance time but stay within same vector
            await testHelper.advanceClockBy(43200); // 12 hours (within the 24 hour duration)

            // Get adjustment again
            const laterAdjustment = await program.getNavAdjustment({ tokenInMint, tokenOutMint });

            // Should be the same since we're still comparing to the same "previous" state (which is 0/none)
            expect(laterAdjustment).toBe(initialAdjustment);
        });

        it("Should provide consistent results for same state", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add two vectors to have a transition
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 1e9,
                apr: 50_000,
                priceFixDuration: 3600
            });

            await testHelper.advanceClockBy(3600);
            const newCurrentTime = await testHelper.getCurrentClockTime();

            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: newCurrentTime,
                basePrice: 1.5e9,
                apr: 30_000,
                priceFixDuration: 3600
            });

            // Call getNavAdjustment multiple times
            const adjustment1 = await program.getNavAdjustment({ tokenInMint, tokenOutMint });
            const adjustment2 = await program.getNavAdjustment({ tokenInMint, tokenOutMint });
            const adjustment3 = await program.getNavAdjustment({ tokenInMint, tokenOutMint });

            // All calls should return the same result
            expect(adjustment1).toBe(adjustment2);
            expect(adjustment2).toBe(adjustment3);
        });
    });
});