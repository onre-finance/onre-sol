import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Update Single Redemption Offer Fee", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints
        tokenInMint = testHelper.createMint(9);
        tokenOutMint = testHelper.createMint(6);

        // Initialize program and offers
        await program.initialize();
        await program.initializeOffers();
    });

    describe("Update Single Redemption Offer Fee Tests", () => {
        it("Should successfully update fee for existing single redemption offer", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Create a single redemption offer first
            await program.makeSingleRedemptionOffer({
                startTime: currentTime + 1000,
                endTime: currentTime + 3600,
                price: 1000,
                feeBasisPoints: 500,
                tokenInMint,
                tokenOutMint
            });

            const offerId = 1;
            const newFee = 1000; // Update to 10%

            // Update the fee
            await program.updateSingleRedemptionOfferFee({ offerId, newFee });

            // Verify the fee was updated
            const offer = await program.getSingleRedemptionOffer(offerId);

            expect(offer).toBeDefined();
            expect(offer.feeBasisPoints.toString()).toBe(newFee.toString());
        });

        it("Should update fee to 0 (free offer)", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Create a single redemption offer first
            await program.makeSingleRedemptionOffer({
                startTime: currentTime + 1000,
                endTime: currentTime + 3600,
                price: 1000,
                feeBasisPoints: 500,
                tokenInMint,
                tokenOutMint
            });

            const offerId = 1;
            const newFee = 0; // Update to 0% (free)

            await program.updateSingleRedemptionOfferFee({ offerId, newFee });

            // Verify the fee was updated to 0
            const offer = await program.getSingleRedemptionOffer(offerId);
            expect(offer.feeBasisPoints.toString()).toBe("0");
        });

        it("Should update fee to maximum (10000 basis points = 100%)", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Create a single redemption offer first
            await program.makeSingleRedemptionOffer({
                startTime: currentTime + 1000,
                endTime: currentTime + 3600,
                price: 1000,
                feeBasisPoints: 500,
                tokenInMint,
                tokenOutMint
            });

            const offerId = 1;
            const newFee = 10000; // Maximum fee (100%)

            await program.updateSingleRedemptionOfferFee({ offerId, newFee });

            // Verify the fee was updated to maximum
            const offer = await program.getSingleRedemptionOffer(offerId);
            expect(offer.feeBasisPoints.toString()).toBe("10000");
        });

        it("Should reject update for non-existent offer", async () => {
            const nonExistentOfferId = 999999;
            const newFee = 1000;

            await expect(
                program.updateSingleRedemptionOfferFee({ offerId: nonExistentOfferId, newFee })
            ).rejects.toThrow("Offer not found");
        });

        it("Should reject zero offer_id", async () => {
            await expect(program.updateSingleRedemptionOfferFee({ offerId: 0, newFee: 1000 })
            ).rejects.toThrow("Offer not found");
        });

        it("Should reject fee greater than 10000 basis points", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Create a single redemption offer first
            await program.makeSingleRedemptionOffer({
                startTime: currentTime + 1000,
                endTime: currentTime + 3600,
                price: 1000,
                feeBasisPoints: 500,
                tokenInMint,
                tokenOutMint
            });

            const offerId = 1;
            const invalidFee = 10001; // Too high (>100%)

            await expect(
                program.updateSingleRedemptionOfferFee({ offerId, newFee: invalidFee })
            ).rejects.toThrow("Invalid fee: fee_basis_points must be <= 10000");
        });

        it("Should reject when called by non-boss", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Create a single redemption offer first
            await program
                .makeSingleRedemptionOffer({
                    startTime: currentTime + 1000,
                    endTime: currentTime + 3600,
                    price: 1000,
                    feeBasisPoints: 500,
                    tokenInMint,
                    tokenOutMint
                });

            const offerId = 1;
            const newFee = 1000;

            await expect(
                program.updateSingleRedemptionOfferFee({ offerId, newFee, signer: testHelper.createUserAccount() })
            ).rejects.toThrow("unknown signer");
        });

        it("Should allow multiple fee updates on same offer", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Create a single redemption offer first
            await program
                .makeSingleRedemptionOffer({
                    startTime: currentTime + 1000,
                    endTime: currentTime + 3600,
                    price: 1000,
                    feeBasisPoints: 500,
                    tokenInMint,
                    tokenOutMint
                });

            const offerId = 1;

            // First update
            await program.updateSingleRedemptionOfferFee({ offerId, newFee: 750 });

            let offer = await program.getSingleRedemptionOffer(offerId);
            expect(offer.feeBasisPoints.toString()).toBe("750");

            // Second update
            await program
                .updateSingleRedemptionOfferFee({ offerId, newFee: 250 });
            offer = await program.getSingleRedemptionOffer(offerId);
            expect(offer.feeBasisPoints.toString()).toBe("250");
        });

        it("Should preserve other offer fields when updating fee", async () => {
            const currentTime = await testHelper.getCurrentClockTime();
            const startTime = currentTime + 1000;
            const endTime = currentTime + 3600;
            const price = 1000;
            const initialFee = 500; // 5% initial fee

            // Create a single redemption offer first
            await program
                .makeSingleRedemptionOffer({
                    startTime,
                    endTime,
                    price,
                    feeBasisPoints: initialFee,
                    tokenInMint,
                    tokenOutMint
                });

            const offerId = 1;
            const newFee = 800;

            // Update the fee
            await program.updateSingleRedemptionOfferFee({ offerId, newFee });

            // Verify fee was updated and other fields remain intact
            const offer = await program.getSingleRedemptionOffer(offerId);

            expect(offer.feeBasisPoints.toString()).toBe("800");
            // Verify other fields remain unchanged
            expect(offer.offerId.toString()).toBe("1");
            expect(offer.tokenInMint.toString()).toBe(tokenInMint.toString());
            expect(offer.tokenOutMint.toString()).toBe(tokenOutMint.toString());
            expect(offer.startTime.toString()).toBe(startTime.toString());
            expect(offer.endTime.toString()).toBe(endTime.toString());
            expect(offer.price.toString()).toBe(price.toString());
        });
    });
});