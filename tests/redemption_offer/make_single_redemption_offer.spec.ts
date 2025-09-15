import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

const MAX_REDEMPTION_OFFERS = 50;

describe("Make single redemption offer", () => {
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

    test("Make single redemption offer should succeed", async () => {
        // given
        const startTime = await testHelper.getCurrentClockTime();
        const endTime = startTime + 3600; // 1 hour later
        const price = 1000;

        // when
        await program.makeSingleRedemptionOffer({
            startTime,
            endTime,
            price,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint
        });

        // then
        const redemptionOfferAccountData = await program.getSingleRedemptionOfferAccount();

        expect(redemptionOfferAccountData.counter.toNumber()).toBe(1);

        const firstOffer = redemptionOfferAccountData.offers[0];
        expect(firstOffer.offerId.toNumber()).toBe(1);
        expect(firstOffer.tokenInMint.toString()).toBe(tokenInMint.toString());
        expect(firstOffer.tokenOutMint.toString()).toBe(tokenOutMint.toString());
        expect(firstOffer.startTime.toNumber()).toBe(startTime);
        expect(firstOffer.endTime.toNumber()).toBe(endTime);
        expect(firstOffer.price.toNumber()).toBe(price);
    });

    test("Make multiple redemption offers should succeed", async () => {
        // given
        const initialData = await program.getSingleRedemptionOfferAccount();
        const initialCounter = initialData.counter.toNumber();

        // when - create first offer
        const startTime1 = await testHelper.getCurrentClockTime();
        const endTime1 = startTime1 + 1800; // 30 minutes
        const price1 = 2000;

        await program.makeSingleRedemptionOffer({
            startTime: startTime1,
            endTime: endTime1,
            price: price1,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint
        });

        // create second offer with different tokens
        const token2In = testHelper.createMint(9);
        const token2Out = testHelper.createMint(9);

        const startTime2 = (await testHelper.getCurrentClockTime()) + 3600; // 1 hour from now
        const endTime2 = startTime2 + 7200; // 2 hours duration
        const price2 = 3000;

        await program.makeSingleRedemptionOffer({
            startTime: startTime2,
            endTime: endTime2,
            price: price2,
            feeBasisPoints: 0,
            tokenInMint: token2In,
            tokenOutMint: token2Out
        });

        // then
        const redemptionOfferAccountData = await program.getSingleRedemptionOfferAccount();

        expect(redemptionOfferAccountData.counter.toNumber()).toBe(initialCounter + 2);

        // Find offers by their auto-generated IDs
        const firstOffer = redemptionOfferAccountData.offers.find(offer =>
            offer.tokenInMint.toString() === tokenInMint.toString() &&
            offer.offerId.toNumber() > initialCounter
        );
        expect(firstOffer).toBeDefined();
        expect(firstOffer!.offerId.toNumber()).toBe(initialCounter + 1);
        expect(firstOffer!.price.toNumber()).toBe(price1);
        expect(firstOffer!.startTime.toNumber()).toBe(startTime1);

        const secondOffer = redemptionOfferAccountData.offers.find(offer =>
            offer.tokenInMint.toString() === token2In.toString()
        );
        expect(secondOffer).toBeDefined();
        expect(secondOffer!.offerId.toNumber()).toBe(initialCounter + 2);
        expect(secondOffer!.price.toNumber()).toBe(price2);
        expect(secondOffer!.startTime.toNumber()).toBe(startTime2);
    });

    test("Make redemption offer with invalid token mints should fail", async () => {
        // when/then
        const startTime = await testHelper.getCurrentClockTime();
        const endTime = startTime + 3600;
        const price = 1000;

        await expect(
            program.makeSingleRedemptionOffer({
                startTime,
                endTime,
                price,
                feeBasisPoints: 0,
                tokenInMint: new PublicKey(0),
                tokenOutMint: new PublicKey(0)
            })
        ).rejects.toThrow();
    });

    test("Make redemption offer with zero price should succeed", async () => {
        // given - zero price should be allowed (free redemption)
        const startTime = await testHelper.getCurrentClockTime();
        const endTime = startTime + 3600;
        const price = 0;

        const uniqueTokenIn = testHelper.createMint(9);
        const uniqueTokenOut = testHelper.createMint(9);

        // when
        await program.makeSingleRedemptionOffer({
            startTime,
            endTime,
            price,
            feeBasisPoints: 0,
            tokenInMint: uniqueTokenIn,
            tokenOutMint: uniqueTokenOut
        });

        // then
        const redemptionOfferAccountData = await program.getSingleRedemptionOfferAccount();

        const zeroOffer = redemptionOfferAccountData.offers.find(offer =>
            offer.tokenInMint.toString() === uniqueTokenIn.toString()
        );
        expect(zeroOffer).toBeDefined();
        expect(zeroOffer!.price.toNumber()).toBe(0);
    });

    test("Make redemption offer with past start time should succeed", async () => {
        // given - past start time should be allowed (immediately active)
        const currentTime = await testHelper.getCurrentClockTime();
        const startTime = currentTime - 3600; // 1 hour ago
        const endTime = currentTime + 3600; // 1 hour from now
        const price = 500;

        const uniqueTokenIn = testHelper.createMint(9);
        const uniqueTokenOut = testHelper.createMint(9);

        // when
        await program.makeSingleRedemptionOffer({
            startTime,
            endTime,
            price,
            feeBasisPoints: 0,
            tokenInMint: uniqueTokenIn,
            tokenOutMint: uniqueTokenOut
        });

        // then
        const redemptionOfferAccountData = await program.getSingleRedemptionOfferAccount();

        const pastOffer = redemptionOfferAccountData.offers.find(offer =>
            offer.tokenInMint.toString() === uniqueTokenIn.toString()
        );
        expect(pastOffer).toBeDefined();
        expect(pastOffer!.startTime.toNumber()).toBe(startTime);
        expect(pastOffer!.endTime.toNumber()).toBe(endTime);
    });

    test("Make more than max redemption offers should fail", async () => {
        // given - check how many offers already exist
        let redemptionOfferAccount = await program.getSingleRedemptionOfferAccount();
        const existingOffers = redemptionOfferAccount.offers.filter(offer => offer.offerId.toNumber() > 0).length;

        // Fill up remaining slots
        const offersToMake = MAX_REDEMPTION_OFFERS - existingOffers;
        const baseTime = await testHelper.getCurrentClockTime();

        for (let i = 0; i < offersToMake; i++) {
            // Create unique mints for each offer to avoid duplicate transaction issues
            const uniqueTokenIn = testHelper.createMint(9);
            const uniqueTokenOut = testHelper.createMint(9);

            const startTime = baseTime + i * 100; // Stagger start times
            const endTime = baseTime + i * 100 + 3600;
            const price = 1000 + i; // Different prices

            await program.makeSingleRedemptionOffer({
                startTime,
                endTime,
                price,
                feeBasisPoints: 0,
                tokenInMint: uniqueTokenIn,
                tokenOutMint: uniqueTokenOut
            });
        }

        // Verify array is full
        redemptionOfferAccount = await program.getSingleRedemptionOfferAccount();
        const activeOffers = redemptionOfferAccount.offers.filter(offer => offer.offerId.toNumber() > 0).length;
        expect(activeOffers).toBe(MAX_REDEMPTION_OFFERS);

        // when - try to make one more offer (should fail)
        const finalTokenIn = testHelper.createMint(9);
        const finalTokenOut = testHelper.createMint(9);
        const finalStartTime = baseTime + 10000;
        const finalEndTime = baseTime + 13600;
        const finalPrice = 9999;

        await expect(
            program.makeSingleRedemptionOffer({
                startTime: finalStartTime,
                endTime: finalEndTime,
                price: finalPrice,
                feeBasisPoints: 0,
                tokenInMint: finalTokenIn,
                tokenOutMint: finalTokenOut
            })
        ).rejects.toThrow("Single redemption offer account is full");
    });

    test("Make redemption offer should fail when not called by boss", async () => {
        // given
        const startTime = await testHelper.getCurrentClockTime();
        const endTime = startTime + 3600;
        const price = 1000;

        const uniqueTokenIn = testHelper.createMint(9);
        const uniqueTokenOut = testHelper.createMint(9);

        // when/then - try to create with different signer
        const notBoss = testHelper.createUserAccount();

        await expect(
            program.makeSingleRedemptionOffer({
                startTime,
                endTime,
                price,
                feeBasisPoints: 0,
                tokenInMint: uniqueTokenIn,
                tokenOutMint: uniqueTokenOut,
                signer: notBoss
            })
        ).rejects.toThrow(); // Should fail due to boss constraint
    });
});