import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Update Dual Redemption Offer Fee", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint1: PublicKey;
    let tokenOutMint2: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints
        tokenInMint = testHelper.createMint(9);
        tokenOutMint1 = testHelper.createMint(9);
        tokenOutMint2 = testHelper.createMint(6);

        await program.initialize();
        await program.initializeOffers();
    });

    describe("Update Dual Redemption Offer Fee Tests", () => {
        it("Should successfully update fee for existing dual redemption offer", async () => {
            const currentTime = await testHelper.getCurrentClockTime();
            const startTime = currentTime + 1000;
            const endTime = currentTime + 3600;
            const price1 = 1500000000; // 1.5 with 9 decimals
            const price2 = 2000000000; // 2.0 with 9 decimals
            const ratioBasisPoints = 8000; // 80% for token_out_1, 20% for token_out_2
            const initialFee = 500; // 5% initial fee

            // Create a dual redemption offer first
            await program.makeDualRedemptionOffer({
                startTime,
                endTime,
                price1,
                price2,
                ratioBasisPoints,
                feeBasisPoints: initialFee,
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2
            });

            const offerId = 1;
            const newFee = 1000; // Update to 10%

            // Update the fee
            await program.updateDualRedemptionOfferFee({ offerId, newFee });

            // Verify the fee was updated
            const dualRedemptionOfferAccount = await program.getDualRedemptionOfferAccount();
            const offer = await program.getDualRedemptionOffer(offerId);

            expect(offer).toBeDefined();
            expect(offer.feeBasisPoints.toNumber()).toBe(newFee);
        });

        it("Should update fee to 0 (free offer)", async () => {
            const currentTime = await testHelper.getCurrentClockTime();
            const startTime = currentTime + 1000;
            const endTime = currentTime + 3600;
            const price1 = 1500000000;
            const price2 = 2000000000;
            const ratioBasisPoints = 8000;
            const initialFee = 500; // 5% initial fee

            // Create a dual redemption offer first
            await program.makeDualRedemptionOffer({
                startTime,
                endTime,
                price1,
                price2,
                ratioBasisPoints,
                feeBasisPoints: initialFee,
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2
            });

            const offerId = 1;
            const newFee = 0; // Update to 0% (free)

            await program.updateDualRedemptionOfferFee({ offerId, newFee });

            // Verify the fee was updated to 0
            const dualRedemptionOfferAccount = await program.getDualRedemptionOfferAccount();
            const offer = await program.getDualRedemptionOffer(offerId);

            expect(offer.feeBasisPoints.toNumber()).toBe(0);
        });

        it("Should update fee to maximum (10000 basis points = 100%)", async () => {
            const currentTime = await testHelper.getCurrentClockTime();
            const startTime = currentTime + 1000;
            const endTime = currentTime + 3600;
            const price1 = 1500000000;
            const price2 = 2000000000;
            const ratioBasisPoints = 8000;
            const initialFee = 500; // 5% initial fee

            // Create a dual redemption offer first
            await program.makeDualRedemptionOffer({
                startTime,
                endTime,
                price1,
                price2,
                ratioBasisPoints,
                feeBasisPoints: initialFee,
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2
            });

            const offerId = 1;
            const newFee = 10000; // Maximum fee (100%)

            await program.updateDualRedemptionOfferFee({ offerId, newFee });

            // Verify the fee was updated to maximum
            const dualRedemptionOfferAccount = await program.getDualRedemptionOfferAccount();
            const offer = dualRedemptionOfferAccount.offers.find(o => o.offerId.toNumber() === offerId);

            expect(offer.feeBasisPoints.toNumber()).toBe(10000);
        });

        it("Should reject update for non-existent offer", async () => {
            const nonExistentOfferId = 999999;
            const newFee = 1000;

            await expect(
                program.updateDualRedemptionOfferFee({ offerId: nonExistentOfferId, newFee })
            ).rejects.toThrow("Offer not found");
        });

        it("Should reject zero offer_id", async () => {
            const zeroOfferId = 0;
            const newFee = 1000;

            await expect(
                program.updateDualRedemptionOfferFee({ offerId: zeroOfferId, newFee })
            ).rejects.toThrow("Offer not found");
        });

        it("Should reject fee greater than 10000 basis points", async () => {
            const currentTime = await testHelper.getCurrentClockTime();
            const startTime = currentTime + 1000;
            const endTime = currentTime + 3600;
            const price1 = 1500000000;
            const price2 = 2000000000;
            const ratioBasisPoints = 8000;
            const initialFee = 500; // 5% initial fee

            // Create a dual redemption offer first
            await program.makeDualRedemptionOffer({
                startTime,
                endTime,
                price1,
                price2,
                ratioBasisPoints,
                feeBasisPoints: initialFee,
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2
            });

            const offerId = 1;
            const invalidFee = 10001; // Too high (>100%)

            await expect(
                program.updateDualRedemptionOfferFee({ offerId, newFee: invalidFee })
            ).rejects.toThrow("Invalid fee: fee_basis_points must be <= 10000");
        });

        it("Should reject when called by non-boss", async () => {
            const currentTime = await testHelper.getCurrentClockTime();
            const startTime = currentTime + 1000;
            const endTime = currentTime + 3600;
            const price1 = 1500000000;
            const price2 = 2000000000;
            const ratioBasisPoints = 8000;
            const initialFee = 500; // 5% initial fee
            const nonBoss = testHelper.createUserAccount();

            // Create a dual redemption offer first
            await expect(program.makeDualRedemptionOffer({
                startTime,
                endTime,
                price1,
                price2,
                ratioBasisPoints,
                feeBasisPoints: initialFee,
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2,
                signer: nonBoss
            })).rejects.toThrow("unknown signer");
        });

        it("Should allow multiple fee updates on same offer", async () => {
            const currentTime = await testHelper.getCurrentClockTime();
            const startTime = currentTime + 1000;
            const endTime = currentTime + 3600;
            const price1 = 1500000000;
            const price2 = 2000000000;
            const ratioBasisPoints = 8000;
            const initialFee = 500; // 5% initial fee

            // Create a dual redemption offer first
            await program.makeDualRedemptionOffer({
                startTime,
                endTime,
                price1,
                price2,
                ratioBasisPoints,
                feeBasisPoints: initialFee,
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2
            });

            const offerId = 1;

            // First update
            const firstNewFee = 750;
            await program.updateDualRedemptionOfferFee({ offerId, newFee: firstNewFee });

            let offer = await program.getDualRedemptionOffer(offerId);
            expect(offer.feeBasisPoints.toNumber()).toBe(750);

            // Second update
            const secondNewFee = 250;
            await program.updateDualRedemptionOfferFee({ offerId, newFee: secondNewFee });

            offer = await program.getDualRedemptionOffer(offerId);
            expect(offer.feeBasisPoints.toNumber()).toBe(250);
        });

        it("Should preserve other offer fields when updating fee", async () => {
            const currentTime = await testHelper.getCurrentClockTime();
            const startTime = currentTime + 1000;
            const endTime = currentTime + 3600;
            const price1 = 1500000000;
            const price2 = 2000000000;
            const ratioBasisPoints = 8000;
            const initialFee = 500; // 5% initial fee

            // Create a dual redemption offer first
            await program.makeDualRedemptionOffer({
                startTime,
                endTime,
                price1,
                price2,
                ratioBasisPoints,
                feeBasisPoints: initialFee,
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2
            });

            const offerId = 1;
            const newFee = 800;

            // Update the fee
            await program.updateDualRedemptionOfferFee({ offerId, newFee });

            // Verify fee was updated and other fields remain intact
            const dualRedemptionOfferAccount = await program.getDualRedemptionOfferAccount();
            const offer = await program.getDualRedemptionOffer(offerId);

            expect(offer.feeBasisPoints.toNumber()).toBe(800);
            // Verify other fields remain unchanged
            expect(offer.offerId.toNumber()).toBe(1);
            expect(offer.tokenInMint.toString()).toBe(tokenInMint.toString());
            expect(offer.tokenOutMint1.toString()).toBe(tokenOutMint1.toString());
            expect(offer.tokenOutMint2.toString()).toBe(tokenOutMint2.toString());
            expect(offer.startTime.toNumber()).toBe(startTime);
            expect(offer.endTime.toNumber()).toBe(endTime);
            expect(offer.price1.toNumber()).toBe(price1);
            expect(offer.price2.toNumber()).toBe(price2);
            expect(offer.ratioBasisPoints.toNumber()).toBe(ratioBasisPoints);
        });
    });
});