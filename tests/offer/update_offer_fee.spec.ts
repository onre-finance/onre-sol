import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Update Offer Fee", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    let offerId: number;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints
        tokenInMint = testHelper.createMint(9);
        tokenOutMint = testHelper.createMint(9);

        await program.initialize({ onycMint: tokenOutMint });
        await program.initializeOffers();

        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint,
            feeBasisPoints: 500
        });

        offerId = 1;
    });

    it("Should successfully update fee for existing offer", async () => {
        const newFee = 1000; // Update to 10%

        // Update the fee
        await program.updateOfferFee({ offerId, newFee });

        // Verify the fee was updated
        const offer = await program.getOffer(offerId);

        expect(offer).toBeDefined();
        expect(offer.feeBasisPoints.toString()).toBe(newFee.toString());
    });

    it("Should update fee to 0 (free offer)", async () => {
        const newFee = 0; // Update to 0% (no fee)

        await program.updateOfferFee({ offerId, newFee });

        // Verify the fee was updated to 0
        const offer = await program.getOffer(offerId);

        expect(offer.feeBasisPoints.toString()).toBe("0");
    });

    it("Should update fee to maximum (10000 basis points = 100%)", async () => {
        const newFee = 10000; // Maximum fee (100%)

        await program.updateOfferFee({ offerId, newFee });

        // Verify the fee was updated to maximum
        const offer = await program.getOffer(offerId);

        expect(offer.feeBasisPoints.toString()).toBe("10000");
    });

    it("Should reject update for non-existent offer", async () => {
        const nonExistentOfferId = 999999;
        const newFee = 1000;

        await expect(
            program.updateOfferFee({ offerId: nonExistentOfferId, newFee })
        ).rejects.toThrow("Offer not found");
    });

    it("Should reject fee greater than 10000 basis points", async () => {
        const invalidFee = 10001; // Too high (>100%)

        await expect(
            program.updateOfferFee({ offerId, newFee: invalidFee })
        ).rejects.toThrow("Invalid fee: fee_basis_points must be <= 10000");
    });

    it("Should reject when called by non-boss", async () => {
        const newFee = 1000;

        await expect(program.updateOfferFee({
            offerId,
            newFee,
            signer: testHelper.createUserAccount()
        })).rejects.toThrow("unknown signer");
    });

    it("Should allow multiple fee updates on same offer", async () => {
        // First update
        await program.updateOfferFee({ offerId, newFee: 750 });

        let offer = await program.getOffer(offerId);
        expect(offer.feeBasisPoints.toString()).toBe("750");

        // Second update
        await program.updateOfferFee({ offerId, newFee: 250 });

        offer = await program.getOffer(offerId);
        expect(offer.feeBasisPoints.toString()).toBe("250");
    });

    it("Should update fee on offer that has vectors", async () => {
        const currentTime = await testHelper.getCurrentClockTime();

        // Add a vector to the offer
        await program
            .addOfferVector({
                offerId,
                startTime: currentTime + 1000,
                startPrice: 1000000, // 1.0 with 6 decimals
                apr: 5000,    // 0.05% APR
                priceFixDuration: 3600     // 1 hour
            });

        // Update the fee
        const newFee = 800;
        await program.updateOfferFee({ offerId, newFee });

        // Verify fee was updated and vector remains intact
        const offer = await program.getOffer(offerId);

        expect(offer.feeBasisPoints.toString()).toBe("800");
        // Verify vector is still there
        const activeVector = offer.vectors.find(v => v.vectorId.toNumber() !== 0);
        expect(activeVector).toBeDefined();
        expect(activeVector.vectorId.toString()).toBe("1");
    });
});