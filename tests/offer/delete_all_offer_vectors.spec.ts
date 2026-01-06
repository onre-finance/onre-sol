import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

const MAX_VECTORS = 10;

describe("Delete All Offer Vectors", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints
        tokenInMint = testHelper.createMint(9);
        tokenOutMint = testHelper.createMint(9);

        // Initialize program
        await program.initialize({ onycMint: tokenOutMint });
    });

    it("Should delete all vectors from an offer with multiple vectors", async () => {
        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add multiple vectors
        for (let i = 1; i <= 5; i++) {
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + (i * 1000),
                basePrice: i * 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });
        }

        // Verify vectors were added
        let offer = await program.getOffer(tokenInMint, tokenOutMint);
        let activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(5);

        // Delete all vectors
        await program.deleteAllOfferVectors(tokenInMint, tokenOutMint);

        // Verify all vectors were deleted
        offer = await program.getOffer(tokenInMint, tokenOutMint);
        activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(0);
    });

    it("Should succeed when offer has no vectors", async () => {
        // Create an offer without any vectors
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        // Verify no vectors initially
        let offer = await program.getOffer(tokenInMint, tokenOutMint);
        let activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(0);

        // Delete all vectors (should succeed as a no-op)
        await program.deleteAllOfferVectors(tokenInMint, tokenOutMint);

        // Still no vectors
        offer = await program.getOffer(tokenInMint, tokenOutMint);
        activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(0);
    });

    it("Should delete past, active, and future vectors", async () => {
        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add vectors at different times
        await program.addOfferVector({
            tokenInMint,
            tokenOutMint,
            baseTime: currentTime + 100, // Will become past
            basePrice: 1000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        await program.addOfferVector({
            tokenInMint,
            tokenOutMint,
            baseTime: currentTime + 200, // Will become active
            basePrice: 2000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        await program.addOfferVector({
            tokenInMint,
            tokenOutMint,
            baseTime: currentTime + 500, // Will remain future
            basePrice: 3000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        // Advance time to make first two vectors past/active
        await testHelper.advanceClockBy(250);

        // Verify vectors exist
        let offer = await program.getOffer(tokenInMint, tokenOutMint);
        let activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(3);

        // Delete all vectors - should succeed for all
        await program.deleteAllOfferVectors(tokenInMint, tokenOutMint);

        // Verify all deleted
        offer = await program.getOffer(tokenInMint, tokenOutMint);
        activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(0);
    });

    it("Should delete all MAX_VECTORS vectors", async () => {
        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Fill all vector slots
        for (let i = 1; i <= MAX_VECTORS; i++) {
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + (i * 1000),
                basePrice: i * 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });
        }

        // Verify all slots filled
        let offer = await program.getOffer(tokenInMint, tokenOutMint);
        let activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(MAX_VECTORS);

        // Delete all
        await program.deleteAllOfferVectors(tokenInMint, tokenOutMint);

        // Verify all deleted
        offer = await program.getOffer(tokenInMint, tokenOutMint);
        activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(0);
    });

    it("Should fail with incorrect token mints", async () => {
        await expect(
            program.deleteAllOfferVectors(
                tokenInMint,
                testHelper.createMint(9)
            )
        ).rejects.toThrow("AnchorError caused by account: offer");

        await expect(
            program.deleteAllOfferVectors(
                testHelper.createMint(9),
                tokenOutMint
            )
        ).rejects.toThrow("AnchorError caused by account: offer");
    });

    it("Should reject when called by non-boss", async () => {
        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add a vector
        await program.addOfferVector({
            tokenInMint,
            tokenOutMint,
            baseTime: currentTime + 1000,
            basePrice: 1000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        const notBoss = testHelper.createUserAccount();

        await expect(
            program.deleteAllOfferVectors(
                tokenInMint,
                tokenOutMint,
                notBoss
            )
        ).rejects.toThrow(); // Should fail due to boss constraint
    });

    it("Should allow adding vectors after deleting all", async () => {
        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add vectors
        await program.addOfferVector({
            tokenInMint,
            tokenOutMint,
            baseTime: currentTime + 1000,
            basePrice: 1000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        // Delete all
        await program.deleteAllOfferVectors(tokenInMint, tokenOutMint);

        // Add new vectors
        await program.addOfferVector({
            tokenInMint,
            tokenOutMint,
            baseTime: currentTime + 2000,
            basePrice: 2000000,
            apr: 7500,
            priceFixDuration: 1800
        });

        // Verify new vector exists
        const offer = await program.getOffer(tokenInMint, tokenOutMint);
        const activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(1);
        expect(activeVectors[0].basePrice.toNumber()).toBe(2000000);
    });
});
