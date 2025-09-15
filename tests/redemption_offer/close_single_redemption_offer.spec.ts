import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Close single redemption offer", () => {
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

    test("Close single redemption offer should succeed", async () => {
        // given - create a redemption offer first
        const currentTime = await testHelper.getCurrentClockTime();

        await program.makeSingleRedemptionOffer({
            startTime: currentTime,
            endTime: currentTime + 3600,
            price: 1000,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint
        });


        // Verify offer was created
        let redemptionOfferAccountData = await program.getSingleRedemptionOfferAccount();
        const offerId = redemptionOfferAccountData.counter.toNumber();

        const createdOffer = await program.getSingleRedemptionOffer(offerId);
        expect(createdOffer).toBeDefined();
        expect(createdOffer!.offerId.toNumber()).toBe(offerId);

        // Count active offers before closing
        const activeOffersBefore = redemptionOfferAccountData.offers.filter(offer => offer.offerId.toNumber() > 0).length;

        // when - close the offer
        await program.closeSingleRedemptionOffer({ offerId });

        // then - verify offer is cleared
        redemptionOfferAccountData = await program.getSingleRedemptionOfferAccount();

        // The offer with the original ID should no longer exist
        const closedOffer = await program.getSingleRedemptionOffer(offerId);
        expect(closedOffer).toBeUndefined();

        // Count active offers after closing - should be one less
        const activeOffersAfter = redemptionOfferAccountData.offers.filter(offer => offer.offerId.toNumber() > 0).length;
        expect(activeOffersAfter).toBe(activeOffersBefore - 1);

        // Counter should remain unchanged (we don't decrease it when closing)
        expect(redemptionOfferAccountData.counter.toNumber()).toBe(offerId);
    });

    test("Close non-existent redemption offer should fail", async () => {
        // when/then - try to close offer that doesn't exist
        const nonExistentOfferId = 9999;

        await expect(
            program
                .closeSingleRedemptionOffer({ offerId: nonExistentOfferId })
        ).rejects.toThrow("Offer not found");
    });

    test("Close redemption offer with zero ID should fail", async () => {
        // when/then - try to close offer with ID 0
        await expect(
            program.closeSingleRedemptionOffer({ offerId: 0 })
        ).rejects.toThrow("Offer not found");
    });

    test("Close redemption offer should fail when not called by boss", async () => {
        // given - create a redemption offer first
        const currentTime = await testHelper.getCurrentClockTime();

        await program
            .makeSingleRedemptionOffer({
                startTime: currentTime,
                endTime: currentTime + 3600,
                price: 1000,
                feeBasisPoints: 0,
                tokenInMint,
                tokenOutMint
            });

        const offerId = 1;

        // when/then - try to close with different signer
        const notBoss = testHelper.createUserAccount();

        await expect(
            program.closeSingleRedemptionOffer({ offerId, signer: notBoss })
        ).rejects.toThrow("unknown signer"); // Should fail due to boss constraint
    });

    test("Close multiple redemption offers should succeed", async () => {
        // given - create multiple redemption offers
        const offers = [];
        const baseTime = await testHelper.getCurrentClockTime();

        for (let i = 0; i < 3; i++) {
            const tokenInMint = testHelper.createMint(9);
            const tokenOutMint = testHelper.createMint(9);
            const price = 1000 + i * 100;

            await program
                .makeSingleRedemptionOffer({
                    startTime: baseTime + i * 100,
                    endTime: baseTime + i * 100 + 3600,
                    price,
                    feeBasisPoints: 0,
                    tokenInMint,
                    tokenOutMint
                });

            offers.push({ tokenInMint, tokenOutMint, price });
        }

        // Get all offer IDs
        let redemptionOfferAccountData = await program.getSingleRedemptionOfferAccount();
        const finalCounter = redemptionOfferAccountData.counter.toNumber();
        const offerIds = [finalCounter - 2, finalCounter - 1, finalCounter]; // Last 3 offers

        // when - close the first and third offers
        await program.closeSingleRedemptionOffer({ offerId: offerIds[0] });
        await program.closeSingleRedemptionOffer({ offerId: offerIds[2] });

        // then - verify correct offers are closed
        redemptionOfferAccountData = await program.getSingleRedemptionOfferAccount();

        // First offer should be closed (cleared)
        const firstOffer = await program.getSingleRedemptionOffer(offerIds[0]);
        expect(firstOffer).toBeUndefined();

        // Second offer should still exist
        const secondOffer = await program.getSingleRedemptionOffer(offerIds[1]);
        expect(secondOffer).toBeDefined();
        expect(secondOffer!.price.toNumber()).toBe(offers[1].price);

        // Third offer should be closed (cleared)
        const thirdOffer = await program.getSingleRedemptionOffer(offerIds[2]);
        expect(thirdOffer).toBeUndefined();
    });
});