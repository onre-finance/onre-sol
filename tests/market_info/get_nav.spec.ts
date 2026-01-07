import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Get NAV", () => {
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
        it("Should successfully get NAV for offer with active vector in first interval", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector with base price and APR
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 1e9, // 1.0 with 9 decimals
                apr: 36_500, // 3.65% APR (scaled by 1M)
                priceFixDuration: 86400 // 1 day
            });

            // Get NAV
            const nav = await program.getNAV({ tokenInMint, tokenOutMint });

            // Validate price
            expect(nav).toBe(1.0001e9);
        });

        it("Should successfully get NAV for offer with multiple vectors", async () => {
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

            // Add second vector
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + (4 * 86400) - 1, // 1 day later
                basePrice: 2e9, // 2.0 with 9 decimals
                apr: 73_000, // 7.3% APR (scaled by 1M)
                priceFixDuration: 86400
            });

            // Advance time to the last interval in first vector, but before second vector is active
            await testHelper.advanceClockBy(3 * 86400);

            // Get NAV
            const nav = await program.getNAV({ tokenInMint, tokenOutMint });

            // Validate price
            expect(nav).toBe(1.0004e9);
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

            // Call getNAV
            await program.getNAV({ tokenInMint, tokenOutMint });

            // Get offer state after
            const offerAfter = await program.getOffer(tokenInMint, tokenOutMint);

            // Should be identical (no state changes)
            expect(offerAfter).toEqual(offerBefore);
        });
    });

    describe("Price Calculation Tests", () => {
        it("Should calculate price after time advancement", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 1e9,
                apr: 36_500, // 3.65% APR
                priceFixDuration: 86400 // 1 day intervals
            });

            // Advance time by 2 hours (should be in 3rd price interval)
            await testHelper.advanceClockBy(86401); // 1 day

            // Should calculate price for 3rd interval
            const nav = await program.getNAV({ tokenInMint, tokenOutMint });
            expect(nav).toBe(1.0002e9);
        });

        it("Should handle 0 APR values", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Test with 0% APR (should maintain base price)
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 2e9, // 2.0 with 9 decimals
                apr: 0, // 0% APR
                priceFixDuration: 86400
            });

            let nav = await program.getNAV({ tokenInMint, tokenOutMint });

            expect(nav).toBe(2e9);

            await testHelper.advanceClockBy(999_999);

            nav = await program.getNAV({ tokenInMint, tokenOutMint });

            expect(nav).toBe(2e9);
        });
    });

    describe("Error Condition Tests", () => {
        it("Should fail with non-existent offer", async () => {
            await expect(program.getNAV({ tokenInMint: testHelper.createMint(6), tokenOutMint }))
                .rejects.toThrow("AnchorError caused by account: offer");

            await expect(program.getNAV({ tokenInMint, tokenOutMint: testHelper.createMint(9) }))
                .rejects.toThrow("AnchorError caused by account: offer");
        });

        it("Should fail when offer has no active vectors", async () => {
            // Don't add any vectors to the offer
            await expect(program.getNAV({ tokenInMint, tokenOutMint }))
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

            await expect(program.getNAV({ tokenInMint, tokenOutMint }))
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
                baseTime: currentTime,
                basePrice: 1e9,
                apr: 36_500,
                priceFixDuration: 3600
            });

            // Advance time to make the first vector active
            await testHelper.advanceClockBy(1800); // 30 minutes

            // Add second vector (more recent) - must have base_time after the first vector
            const newCurrentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: newCurrentTime,
                basePrice: 2e9, // different base price
                apr: 73_000, // different APR
                priceFixDuration: 1800
            });

            // Should use the more recent vector without errors
            await expect(program.getNAV({ tokenInMint, tokenOutMint })).resolves.not.toThrow();
        });
    });
});