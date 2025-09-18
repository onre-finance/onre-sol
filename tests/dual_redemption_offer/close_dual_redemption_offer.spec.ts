import { Keypair } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Close dual redemption offer", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Initialize program state and offers
        await program.initialize();
        await program.initializeOffers();
    });

    test("Close dual redemption offer should succeed", async () => {
        // given - create a dual redemption offer first
        const tokenInMint = testHelper.createMint(9);
        const tokenOutMint1 = testHelper.createMint(9);
        const tokenOutMint2 = testHelper.createMint(6);

        const startTime = Math.floor(Date.now() / 1000);
        const endTime = startTime + 3600;
        const price1 = 1500000000;
        const price2 = 2000000000;
        const ratioBasisPoints = 8000;

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // Get the created offer ID
        const beforeCloseData = await program.getDualRedemptionOfferAccount();
        const createdOffer = beforeCloseData.offers.find(offer =>
            offer.tokenInMint.toString() === tokenInMint.toString()
        );
        expect(createdOffer).toBeDefined();
        const offerId = createdOffer!.offerId.toNumber();

        // when - close the offer
        await program.closeDualRedemptionOffer({ offerId });

        // then - verify offer is cleared
        const afterCloseData = await program.getDualRedemptionOfferAccount();

        // Count active offers before and after (active = offerId != 0)
        const activeOffersBefore = beforeCloseData.offers.filter(offer => offer.offerId.toNumber() !== 0).length;
        const activeOffersAfter = afterCloseData.offers.filter(offer => offer.offerId.toNumber() !== 0).length;

        // Should be one less active offer
        expect(activeOffersAfter).toBe(activeOffersBefore - 1);

        // Cannot find the specific offer by ID anymore (because ID is now 0)
        const closedOfferSearch = afterCloseData.offers.find(offer => offer.offerId.toNumber() === offerId);
        expect(closedOfferSearch).toBeUndefined();
    });

    test("Close multiple dual redemption offers should succeed", async () => {
        // given - create two dual redemption offers
        const token1InMint = testHelper.createMint(9);
        const token1OutMint1 = testHelper.createMint(9);
        const token1OutMint2 = testHelper.createMint(6);

        const token2InMint = testHelper.createMint(18);
        const token2OutMint1 = testHelper.createMint(9);
        const token2OutMint2 = testHelper.createMint(6);

        const startTime = Math.floor(Date.now() / 1000);
        const endTime = startTime + 3600;
        const price1 = 1000000000;
        const price2 = 500000000;

        // Create first offer
        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints: 7000,
            feeBasisPoints: 0,
            tokenInMint: token1InMint,
            tokenOutMint1: token1OutMint1,
            tokenOutMint2: token1OutMint2
        });

        // Create second offer
        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints: 9000,
            feeBasisPoints: 0,
            tokenInMint: token2InMint,
            tokenOutMint1: token2OutMint1,
            tokenOutMint2: token2OutMint2
        });

        // Get offer IDs
        const beforeCloseData = await program.getDualRedemptionOfferAccount();

        const offer1 = beforeCloseData.offers.find(offer =>
            offer.tokenInMint.toString() === token1InMint.toString()
        );
        const offer2 = beforeCloseData.offers.find(offer =>
            offer.tokenInMint.toString() === token2InMint.toString()
        );

        expect(offer1).toBeDefined();
        expect(offer2).toBeDefined();

        const offerId1 = offer1!.offerId.toNumber();
        const offerId2 = offer2!.offerId.toNumber();

        // when - close both offers
        await program.closeDualRedemptionOffer({ offerId: offerId1 });
        await program.closeDualRedemptionOffer({ offerId: offerId2 });

        // then - verify both offers are cleared
        const afterCloseData = await program.getDualRedemptionOfferAccount();

        const closedOffer1 = afterCloseData.offers.find(offer =>
            offer.offerId.toNumber() === offerId1
        );
        const closedOffer2 = afterCloseData.offers.find(offer =>
            offer.offerId.toNumber() === offerId2
        );

        // Both should be cleared (offerId = 0)
        expect(closedOffer1?.offerId.toNumber()).toBeUndefined();
        expect(closedOffer2?.offerId.toNumber()).toBeUndefined();
    });

    test("Close dual redemption offer with invalid offer ID should fail", async () => {
        // when/then - try to close non-existent offer
        await expect(
            program.closeDualRedemptionOffer({ offerId: 99999 })
        ).rejects.toThrow("Offer not found");
    });

    test("Close dual redemption offer with zero offer ID should fail", async () => {
        // when/then - try to close with zero ID
        await expect(
            program.closeDualRedemptionOffer({ offerId: 0 })
        ).rejects.toThrow("Offer not found");
    });

    test("Close dual redemption offer should fail when not called by boss", async () => {
        // given - create a dual redemption offer first
        const tokenInMint = testHelper.createMint(9);
        const tokenOutMint1 = testHelper.createMint(9);
        const tokenOutMint2 = testHelper.createMint(6);

        const startTime = Math.floor(Date.now() / 1000);
        const endTime = startTime + 3600;
        const price1 = 1000000000;
        const price2 = 500000000;

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints: 8000,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // Get the offer ID
        const data = await program.getDualRedemptionOfferAccount();
        const offer = data.offers.find(offer =>
            offer.tokenInMint.toString() === tokenInMint.toString()
        );
        const offerId = offer!.offerId.toNumber();

        // when/then - try to close with different signer
        const fakeUser = Keypair.generate();

        await expect(
            program.closeDualRedemptionOffer({ offerId, signer: fakeUser })
        ).rejects.toThrow();
    });

    test("Close already closed dual redemption offer should fail", async () => {
        // given - create and close a dual redemption offer
        const tokenInMint = testHelper.createMint(9);
        const tokenOutMint1 = testHelper.createMint(9);
        const tokenOutMint2 = testHelper.createMint(6);

        const startTime = Math.floor(Date.now() / 1000);
        const endTime = startTime + 3600;
        const price1 = 1000000000;
        const price2 = 500000000;

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints: 8000,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // Get the offer ID
        const data = await program.getDualRedemptionOfferAccount();
        const offer = data.offers.find(offer =>
            offer.tokenInMint.toString() === tokenInMint.toString()
        );
        const offerId = offer!.offerId.toNumber();

        // Close the offer once
        await program.closeDualRedemptionOffer({ offerId });

        // when/then - try to close the same offer again (should fail)
        // Add small delay to ensure unique transaction
        await new Promise(resolve => setTimeout(resolve, 10));

        await expect(
            program.closeDualRedemptionOffer({ offerId })
        ).rejects.toThrow("Offer not found");
    });

    test("Verify dual redemption offer counter remains unchanged after closing", async () => {
        // given - get initial counter
        const initialData = await program.getDualRedemptionOfferAccount();
        const initialCounter = initialData.counter.toNumber();

        // Create an offer
        const tokenInMint = testHelper.createMint(9);
        const tokenOutMint1 = testHelper.createMint(9);
        const tokenOutMint2 = testHelper.createMint(6);

        const startTime = Math.floor(Date.now() / 1000);
        const endTime = startTime + 3600;
        const price1 = 1000000000;
        const price2 = 500000000;

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints: 8000,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // Get the offer ID and close it
        const afterCreateData = await program.getDualRedemptionOfferAccount();
        const offer = afterCreateData.offers.find(offer =>
            offer.tokenInMint.toString() === tokenInMint.toString()
        );
        const offerId = offer!.offerId.toNumber();

        await program.closeDualRedemptionOffer({ offerId });

        // then - verify counter is still incremented (not decremented)
        const finalData = await program.getDualRedemptionOfferAccount();
        expect(finalData.counter.toNumber()).toBe(initialCounter + 1);

        // Verify we can still use the same array slot for a new offer
        const newTokenInMint = testHelper.createMint(9);
        const newTokenOutMint1 = testHelper.createMint(9);
        const newTokenOutMint2 = testHelper.createMint(6);

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints: 5000,
            feeBasisPoints: 0,
            tokenInMint: newTokenInMint,
            tokenOutMint1: newTokenOutMint1,
            tokenOutMint2: newTokenOutMint2
        });

        const afterNewOfferData = await program.getDualRedemptionOfferAccount();
        expect(afterNewOfferData.counter.toNumber()).toBe(initialCounter + 2);
    });
});