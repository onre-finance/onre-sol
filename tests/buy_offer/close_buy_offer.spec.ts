import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Close buy offer", () => {
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

        // Initialize program and offers
        await program.initialize();
        await program.initializeOffers();
    });

    test("Close buy offer should succeed and clear the offer", async () => {
        // given - create an offer first
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        // verify offer exists
        let buyOfferAccount = await program.getBuyOfferAccount();
        const offerCounter = buyOfferAccount.counter.toNumber();
        const createdOffer = await program.getOffer(offerCounter);
        expect(createdOffer).toBeDefined();

        // when - close the offer
        await program.closeBuyOffer({ offerId: offerCounter });

        // then - verify offer is cleared
        buyOfferAccount = await program.getBuyOfferAccount();

        // Counter should remain the same (not decremented)
        expect(buyOfferAccount.counter.toNumber()).toBe(offerCounter);

        // Find the offer that was closed - should be cleared
        const closedOffer = await program.getOffer(offerCounter);
        expect(closedOffer).toBeUndefined(); // Should not exist anymore

        // Verify all offers are cleared (since this is the only one)
        const activeOffers = buyOfferAccount.offers.filter(offer => offer.offerId.toNumber() > 0);
        expect(activeOffers.length).toBe(0);
    });

    test("Close buy offer should clear specific offer without affecting others", async () => {
        // given - create multiple offers
        const initialData = await program.getBuyOfferAccount();
        const startingCounter = initialData.counter.toNumber();

        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const token2In = testHelper.createMint(9);
        const token2Out = testHelper.createMint(9);

        await program.makeBuyOffer({
            tokenInMint: token2In,
            tokenOutMint: token2Out
        });

        const token3In = testHelper.createMint(9);
        const token3Out = testHelper.createMint(9);

        await program.makeBuyOffer({
            tokenInMint: token3In,
            tokenOutMint: token3Out
        });

        // verify all offers exist
        let buyOfferAccountData = await program.getBuyOfferAccount();
        expect(buyOfferAccountData.counter.toNumber()).toBe(startingCounter + 3);
        let activeOffers = buyOfferAccountData.offers.filter(offer => offer.offerId.toNumber() > startingCounter);
        expect(activeOffers.length).toBe(3);

        // when - close the middle offer
        const middleOfferId = startingCounter + 2;
        await program.closeBuyOffer({ offerId: middleOfferId });

        // then - verify only the middle offer is cleared
        buyOfferAccountData = await program.getBuyOfferAccount();

        // Counter remains the same
        expect(buyOfferAccountData.counter.toNumber()).toBe(startingCounter + 3);

        // Only 2 active offers remain
        activeOffers = buyOfferAccountData.offers.filter(offer => offer.offerId.toNumber() > startingCounter);
        expect(activeOffers.length).toBe(2);

        // First and third offers should still exist
        const firstOffer = await program.getOffer(startingCounter + 1);
        const thirdOffer = await program.getOffer(startingCounter + 3);

        expect(firstOffer).toBeDefined();
        expect(thirdOffer).toBeDefined();

        // Middle offer should be cleared
        const middleOffer = await program.getOffer(middleOfferId);
        expect(middleOffer).toBeUndefined();
    });

    test("Close buy offer with offer_id 0 should fail", async () => {
        // when/then - try to close with invalid offer_id = 0
        const invalidOfferId = 0;
        await expect(
            program.closeBuyOffer({ offerId: invalidOfferId })
        ).rejects.toThrow("Offer not found");
    });

    test("Close non-existent buy offer should fail", async () => {
        // when/then - try to close non-existent offer (doesn't matter how many other offers exist)
        const nonExistentOfferId = 999;
        await expect(
            program.closeBuyOffer({ offerId: nonExistentOfferId })
        ).rejects.toThrow("Offer not found");
    });

    test("Close buy offer should fail when not called by boss", async () => {
        // given - create an offer
        const initialData = await program.getBuyOfferAccount();
        const startingCounter = initialData.counter.toNumber();

        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const newOfferId = startingCounter + 1;

        // when/then - try to close with different signer
        const notBoss = testHelper.createUserAccount();

        await expect(
            program.closeBuyOffer({ offerId: newOfferId, signer: notBoss })
        ).rejects.toThrow("unknown signer"); // Should fail due to boss constraint
    });
});