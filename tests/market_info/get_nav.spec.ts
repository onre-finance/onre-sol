import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Get NAV", () => {
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
        await program.initialize({ onycMint: tokenOutMint });
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
        it("Should successfully get NAV for offer with active vector in first interval", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector with base price and APR
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9, // 1.0 with 9 decimals
                apr: 36_500, // 3.65% APR (scaled by 1M)
                priceFixDuration: 86400 // 1 day
            });

            // Get NAV
            const nav = await program.getNAV({ offerId });

            // Validate price
            expect(nav).toBe(1.0001e9);
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

            // Call getNAV
            await program.getNAV({ offerId });

            // Get offer state after
            const offerAfter = await program.getOffer(offerId);

            // Should be identical (no state changes)
            expect(offerAfter).toEqual(offerBefore);
        });
    });

    describe("Price Calculation Tests", () => {
        it("Should calculate price after time advancement", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500, // 3.65% APR
                priceFixDuration: 86400 // 1 day intervals
            });

            // Advance time by 2 hours (should be in 3rd price interval)
            await testHelper.advanceClockBy(86401); // 1 day

            // Should calculate price for 3rd interval
            const nav = await program.getNAV({ offerId });
            expect(nav).toBe(1.0002e9);
        });

        it("Should handle 0 APR values", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Test with 0% APR (should maintain base price)
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 2e9, // 2.0 with 9 decimals
                apr: 0, // 0% APR
                priceFixDuration: 86400
            });

            let nav = await program.getNAV({ offerId });

            expect(nav).toBe(2e9);

            await testHelper.advanceClockBy(999_999);

            nav = await program.getNAV({ offerId });

            expect(nav).toBe(2e9);
        });
    });

    describe("Error Condition Tests", () => {
        it("Should fail with non-existent offer ID", async () => {
            const nonExistentOfferId = 999;

            await expect(program.getNAV({ offerId: nonExistentOfferId }))
                .rejects.toThrow("Offer not found");
        });

        it("Should fail with invalid offer ID (0)", async () => {
            await expect(program.getNAV({ offerId: 0 }))
                .rejects.toThrow("Offer not found");
        });

        it("Should fail when offer has no active vectors", async () => {
            // Don't add any vectors to the offer
            await expect(program.getNAV({ offerId }))
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

            await expect(program.getNAV({ offerId }))
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
                apr: 36_500,
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
                apr: 73_000, // different APR
                priceFixDuration: 1800
            });

            // Should use the more recent vector without errors
            await expect(program.getNAV({ offerId })).resolves.not.toThrow();
        });
    });
});