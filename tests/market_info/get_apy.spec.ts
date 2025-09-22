import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Get APY", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    let offerId: number;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints with different decimals to test precision handling
        tokenInMint = testHelper.createMint(6); // USDC-like (6 decimals)
        tokenOutMint = testHelper.createMint(9); // ONyc-like (9 decimals)

        // Initialize program and offers
        await program.initialize();
        await program.initializeOffers();

        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        const offerAccount = await program.getOfferAccount();
        const offer = offerAccount.offers.find(o => o.offerId.toNumber() !== 0);
        offerId = offer.offerId.toNumber();
    });

    describe("Basic Functionality Tests", () => {
        it("Should successfully get APY for offer with active vector", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector with 3.65% APR (scaled by 1M)
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9, // 1.0 with 9 decimals
                apr: 36_500, // 3.65% APR (scaled by 1M)
                priceFixDuration: 86400 // 1 day
            });

            // Get APY
            const apy = await program.getAPY({ offerId });

            // APY should be slightly higher than APR due to compounding
            expect(apy).toBe(37_172);
        });

        it("Should be read-only instruction (no state changes)", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Get offer state before
            const offerBefore = await program.getOffer(offerId);

            // Call getAPY
            await program.getAPY({ offerId });

            // Get offer state after
            const offerAfter = await program.getOffer(offerId);

            // Should be identical (no state changes)
            expect(offerAfter).toEqual(offerBefore);
        });
    });

    describe("APR to APY Conversion Tests", () => {
        it("Should handle 0% APR correctly (should return 0% APY)", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Test with 0% APR
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 2e9, // 2.0 with 9 decimals
                apr: 0, // 0% APR
                priceFixDuration: 86400
            });

            const apy = await program.getAPY({ offerId });

            expect(apy).toBe(0);
        });

        it("Should calculate correct APY for 10% APR", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Test with 10% APR (scaled by 1M)
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 100_000, // 10% APR (scaled by 1M)
                priceFixDuration: 86400
            });

            const apy = await program.getAPY({ offerId });

            expect(apy).toBe(105156); // Should be higher than APR
        });

        it("Should calculate correct APY for higher APR values", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Test with 25% APR (scaled by 1M)
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 250_000, // 25% APR (scaled by 1M)
                priceFixDuration: 86400
            });

            const apy = await program.getAPY({ offerId });

            expect(apy).toBe(283916);
        });

        it("Should handle very small APR values", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Test with 0.01% APR (scaled by 1M)
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 100, // 0.01% APR (scaled by 1M)
                priceFixDuration: 86400
            });

            const apy = await program.getAPY({ offerId });

            // For very small APR, APY should be essentially the same
            expect(apy).toBe(100);
        });
    });

    describe("Error Condition Tests", () => {
        it("Should fail with non-existent offer ID", async () => {
            const nonExistentOfferId = 999;

            await expect(program.getAPY({ offerId: nonExistentOfferId }))
                .rejects.toThrow("Offer not found");
        });

        it("Should fail with invalid offer ID (0)", async () => {
            await expect(program.getAPY({ offerId: 0 }))
                .rejects.toThrow("Offer not found");
        });

        it("Should fail when offer has no active vectors", async () => {
            // Don't add any vectors to the offer
            await expect(program.getAPY({ offerId }))
                .rejects.toThrow("No active vector");
        });

        it("Should fail when no vector is active at current time", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector that starts in the future
            await program.addOfferVector({
                offerId,
                startTime: currentTime + 86400, // starts tomorrow
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            await expect(program.getAPY({ offerId }))
                .rejects.toThrow("No active vector");
        });
    });

    describe("Edge Case Tests", () => {
        it("Should handle multiple vectors and use most recent active one", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add first vector (older) - use current time as base since vectors must be in order
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 50_000, // 5% APR
                priceFixDuration: 3600
            });

            // Advance time to make the first vector active
            await testHelper.advanceClockBy(1800); // 30 minutes

            // Add second vector (more recent) - must have base_time after the first vector
            const newCurrentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                offerId,
                startTime: newCurrentTime,
                startPrice: 2e9, // different base price
                apr: 100_000, // 10% APR (different from first vector)
                priceFixDuration: 1800
            });

            // Should use the more recent vector (10% APR) for APY calculation
            const apy = await program.getAPY({ offerId });

            // Should be based on 10% APR, not 5% APR
            expect(apy).toBe(105156); // Higher than 10% APR
        });

        it("Should provide consistent results for same APR", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 75_000, // 7.5% APR
                priceFixDuration: 86400
            });

            // Call getAPY multiple times
            const apy1 = await program.getAPY({ offerId });
            const apy2 = await program.getAPY({ offerId });
            const apy3 = await program.getAPY({ offerId });

            // All calls should return the same result
            expect(apy1).toBe(apy2);
            expect(apy2).toBe(apy3);
        });
    });
});