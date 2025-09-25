import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Get APY", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

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

        const offer = await program.getOffer(tokenInMint, tokenOutMint);
        expect(offer.tokenInMint).toStrictEqual(tokenInMint);
        expect(offer.tokenOutMint).toStrictEqual(tokenOutMint);
    });

    describe("Basic Functionality Tests", () => {
        it("Should successfully get APY for offer with active vector", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector with 3.65% APR (scaled by 1M)
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                startTime: currentTime,
                startPrice: 1e9, // 1.0 with 9 decimals
                apr: 36_500, // 3.65% APR (scaled by 1M)
                priceFixDuration: 86400 // 1 day
            });

            // Get APY
            const apy = await program.getAPY({ tokenInMint, tokenOutMint });

            // APY should be slightly higher than APR due to compounding
            expect(apy).toBe(37_172);
        });

        it("Should be read-only instruction (no state changes)", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Get offer state before
            const offerBefore = await program.getOffer(tokenInMint, tokenOutMint);

            // Call getAPY
            await program.getAPY({ tokenInMint, tokenOutMint });

            // Get offer state after
            const offerAfter = await program.getOffer(tokenInMint, tokenOutMint);

            // Should be identical (no state changes)
            expect(offerAfter).toEqual(offerBefore);
        });
    });

    describe("APR to APY Conversion Tests", () => {
        it("Should handle 0% APR correctly (should return 0% APY)", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Test with 0% APR
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                startTime: currentTime,
                startPrice: 2e9, // 2.0 with 9 decimals
                apr: 0, // 0% APR
                priceFixDuration: 86400
            });

            const apy = await program.getAPY({ tokenInMint, tokenOutMint });

            expect(apy).toBe(0);
        });

        it("Should calculate correct APY for 10% APR", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Test with 10% APR (scaled by 1M)
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 100_000, // 10% APR (scaled by 1M)
                priceFixDuration: 86400
            });

            const apy = await program.getAPY({ tokenInMint, tokenOutMint });

            expect(apy).toBe(105156); // Should be higher than APR
        });

        it("Should calculate correct APY for higher APR values", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Test with 25% APR (scaled by 1M)
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 250_000, // 25% APR (scaled by 1M)
                priceFixDuration: 86400
            });

            const apy = await program.getAPY({ tokenInMint, tokenOutMint });

            expect(apy).toBe(283916);
        });

        it("Should handle very small APR values", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Test with 0.01% APR (scaled by 1M)
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 100, // 0.01% APR (scaled by 1M)
                priceFixDuration: 86400
            });

            const apy = await program.getAPY({ tokenInMint, tokenOutMint });

            // For very small APR, APY should be essentially the same
            expect(apy).toBe(100);
        });
    });

    describe("Error Condition Tests", () => {
        it("Should fail with non-existent offer", async () => {
            await expect(program.getAPY({ tokenInMint, tokenOutMint: testHelper.createMint(9) }))
                .rejects.toThrow("AnchorError caused by account: offer");

            await expect(program.getAPY({ tokenInMint, tokenOutMint: testHelper.createMint(9) }))
                .rejects.toThrow("AnchorError caused by account: offer");
        });

        it("Should fail when offer has no active vectors", async () => {
            // Don't add any vectors to the offer
            await expect(program.getAPY({ tokenInMint, tokenOutMint }))
                .rejects.toThrow("No active vector");
        });

        it("Should fail when no vector is active at current time", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector that starts in the future
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                startTime: currentTime + 86400, // starts tomorrow
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            await expect(program.getAPY({ tokenInMint, tokenOutMint }))
                .rejects.toThrow("No active vector");
        });
    });

    describe("Edge Case Tests", () => {
        it("Should handle multiple vectors and use most recent active one", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add first vector (older) - use current time as base since vectors must be in order
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
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
                tokenInMint,
                tokenOutMint,
                startTime: newCurrentTime,
                startPrice: 2e9, // different base price
                apr: 100_000, // 10% APR (different from first vector)
                priceFixDuration: 1800
            });

            // Should use the more recent vector (10% APR) for APY calculation
            const apy = await program.getAPY({ tokenInMint, tokenOutMint });

            // Should be based on 10% APR, not 5% APR
            expect(apy).toBe(105156); // Higher than 10% APR
        });

        it("Should provide consistent results for same APR", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 75_000, // 7.5% APR
                priceFixDuration: 86400
            });

            // Call getAPY multiple times
            const apy1 = await program.getAPY({ tokenInMint, tokenOutMint });
            const apy2 = await program.getAPY({ tokenInMint, tokenOutMint });
            const apy3 = await program.getAPY({ tokenInMint, tokenOutMint });

            // All calls should return the same result
            expect(apy1).toBe(apy2);
            expect(apy2).toBe(apy3);
        });
    });
});