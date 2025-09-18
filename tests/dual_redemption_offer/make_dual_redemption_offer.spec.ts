import { Keypair } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

const MAX_DUAL_REDEMPTION_OFFERS = 50;

describe("Make dual redemption offer", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Initialize program state and offers
        await program.initialize();
        await program.initializeOffers();
    });

    test("Make a dual redemption offer should succeed", async () => {
        // given
        const tokenInMint = testHelper.createMint(9);
        const tokenOutMint1 = testHelper.createMint(9);
        const tokenOutMint2 = testHelper.createMint(6);

        const startTime = Math.floor(Date.now() / 1000) - 60; // 1 minute ago
        const endTime = startTime + 3600; // 1 hour from start
        const price1 = 1500000000; // 1.5 with 9 decimals
        const price2 = 2000000000; // 2.0 with 9 decimals
        const ratioBasisPoints = 8000; // 80% for token_out_1, 20% for token_out_2

        // when
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

        // then
        const dualRedemptionOfferAccountData = await program.getDualRedemptionOfferAccount();

        expect(dualRedemptionOfferAccountData.counter.toNumber()).toBe(1);

        const firstOffer = dualRedemptionOfferAccountData.offers[0];
        expect(firstOffer.offerId.toNumber()).toBe(1);
        expect(firstOffer.tokenInMint.toString()).toBe(tokenInMint.toString());
        expect(firstOffer.tokenOutMint1.toString()).toBe(tokenOutMint1.toString());
        expect(firstOffer.tokenOutMint2.toString()).toBe(tokenOutMint2.toString());
        expect(firstOffer.price1.toNumber()).toBe(price1);
        expect(firstOffer.price2.toNumber()).toBe(price2);
        expect(firstOffer.ratioBasisPoints.toNumber()).toBe(ratioBasisPoints);
        expect(firstOffer.startTime.toNumber()).toBe(startTime);
        expect(firstOffer.endTime.toNumber()).toBe(endTime);
    });

    test("Make multiple dual redemption offers should succeed", async () => {
        // given
        const initialData = await program.getDualRedemptionOfferAccount();
        const initialCounter = initialData.counter.toNumber();

        // when - make first offer
        const token1In = testHelper.createMint(9);
        const token1Out1 = testHelper.createMint(9);
        const token1Out2 = testHelper.createMint(6);

        const startTime1 = Math.floor(Date.now() / 1000);
        const endTime1 = startTime1 + 3600;
        const price1_1 = 1000000000; // 1.0
        const price1_2 = 500000000; // 0.5
        const ratio1 = 7000; // 70/30 split

        await program.makeDualRedemptionOffer({
            startTime: startTime1,
            endTime: endTime1,
            price1: price1_1,
            price2: price1_2,
            ratioBasisPoints: ratio1,
            feeBasisPoints: 0,
            tokenInMint: token1In,
            tokenOutMint1: token1Out1,
            tokenOutMint2: token1Out2
        });

        // make second offer
        const token2In = testHelper.createMint(18);
        const token2Out1 = testHelper.createMint(9);
        const token2Out2 = testHelper.createMint(6);

        const startTime2 = Math.floor(Date.now() / 1000);
        const endTime2 = startTime2 + 7200;
        const price2_1 = 2500000000; // 2.5
        const price2_2 = 1200000000; // 1.2
        const ratio2 = 9000; // 90/10 split

        await program.makeDualRedemptionOffer({
            startTime: startTime2,
            endTime: endTime2,
            price1: price2_1,
            price2: price2_2,
            ratioBasisPoints: ratio2,
            feeBasisPoints: 0,
            tokenInMint: token2In,
            tokenOutMint1: token2Out1,
            tokenOutMint2: token2Out2
        });

        // then
        const dualRedemptionOfferAccountData = await program.getDualRedemptionOfferAccount();

        expect(dualRedemptionOfferAccountData.counter.toNumber()).toBe(initialCounter + 2);

        // Find offers by their properties
        const firstOffer = await program.getDualRedemptionOffer(1);
        expect(firstOffer).toBeDefined();
        expect(firstOffer!.offerId.toNumber()).toBe(initialCounter + 1);
        expect(firstOffer!.tokenOutMint1.toString()).toBe(token1Out1.toString());
        expect(firstOffer!.tokenOutMint2.toString()).toBe(token1Out2.toString());
        expect(firstOffer!.ratioBasisPoints.toNumber()).toBe(ratio1);

        const secondOffer = await program.getDualRedemptionOffer(2);
        expect(secondOffer).toBeDefined();
        expect(secondOffer!.offerId.toNumber()).toBe(initialCounter + 2);
        expect(secondOffer!.tokenOutMint1.toString()).toBe(token2Out1.toString());
        expect(secondOffer!.tokenOutMint2.toString()).toBe(token2Out2.toString());
        expect(secondOffer!.ratioBasisPoints.toNumber()).toBe(ratio2);
    });

    test("Make dual redemption offer with invalid ratio should fail", async () => {
        // given
        const tokenInMint = testHelper.createMint(9);
        const tokenOutMint1 = testHelper.createMint(9);
        const tokenOutMint2 = testHelper.createMint(6);

        const startTime = Math.floor(Date.now() / 1000);
        const endTime = startTime + 3600;
        const price1 = 1000000000;
        const price2 = 500000000;
        const invalidRatio = 10001; // > 10000 (100%)

        // when/then
        await expect(
            program.makeDualRedemptionOffer({
                startTime,
                endTime,
                price1,
                price2,
                ratioBasisPoints: invalidRatio,
                feeBasisPoints: 0,
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2
            })
        ).rejects.toThrow("Invalid ratio");
    });

    test("Make dual redemption offer with edge case ratios should succeed", async () => {
        // Test 0% ratio (all goes to token_out_2)
        const tokenInMint1 = testHelper.createMint(9);
        const tokenOutMint1_1 = testHelper.createMint(9);
        const tokenOutMint1_2 = testHelper.createMint(6);

        const startTime = Math.floor(Date.now() / 1000);
        const endTime = startTime + 3600;
        const price1 = 1000000000;
        const price2 = 500000000;

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints: 0, // 0% for token_out_1
            feeBasisPoints: 0,
            tokenInMint: tokenInMint1,
            tokenOutMint1: tokenOutMint1_1,
            tokenOutMint2: tokenOutMint1_2
        });

        // Test 100% ratio (all goes to token_out_1)
        const tokenInMint2 = testHelper.createMint(9);
        const tokenOutMint2_1 = testHelper.createMint(9);
        const tokenOutMint2_2 = testHelper.createMint(6);

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints: 10000, // 100% for token_out_1
            feeBasisPoints: 0,
            tokenInMint: tokenInMint2,
            tokenOutMint1: tokenOutMint2_1,
            tokenOutMint2: tokenOutMint2_2
        });

        // Verify both offers were created
        const offer0 = await program.getDualRedemptionOffer(1);
        expect(offer0!.ratioBasisPoints.toNumber()).toBe(0);

        const offer100 = await program.getDualRedemptionOffer(2);
        expect(offer100!.ratioBasisPoints.toNumber()).toBe(10000);
    });

    test("Make dual redemption offer should fail when not called by boss", async () => {
        // given
        const startTime = Math.floor(Date.now() / 1000);
        const endTime = startTime + 3600;
        const price1 = 1000000000;
        const price2 = 500000000;

        const uniqueTokenIn = testHelper.createMint(9);
        const uniqueTokenOut1 = testHelper.createMint(9);
        const uniqueTokenOut2 = testHelper.createMint(6);

        // when/then - try to create with different signer
        const fakeUser = Keypair.generate();

        await expect(
            program.makeDualRedemptionOffer({
                startTime,
                endTime,
                price1,
                price2,
                ratioBasisPoints: 8000,
                feeBasisPoints: 0,
                tokenInMint: uniqueTokenIn,
                tokenOutMint1: uniqueTokenOut1,
                tokenOutMint2: uniqueTokenOut2,
                signer: fakeUser
            })
        ).rejects.toThrow();
    });

    test("Make more than max dual redemption offers should fail", async () => {
        // given - check how many offers already exist
        let dualRedemptionOfferAccount = await program.getDualRedemptionOfferAccount();
        const existingOffers = dualRedemptionOfferAccount.offers.filter(offer => offer.offerId.toNumber() > 0).length;

        // Fill up remaining slots
        const offersToMake = MAX_DUAL_REDEMPTION_OFFERS - existingOffers;
        const startTime = Math.floor(Date.now() / 1000);
        const endTime = startTime + 3600;
        const price1 = 1000000000;
        const price2 = 500000000;

        for (let i = 0; i < offersToMake; i++) {
            try {
                // Create unique mints for each offer to avoid duplicate transaction issues
                const uniqueTokenIn = testHelper.createMint(9);
                const uniqueTokenOut1 = testHelper.createMint(9);
                const uniqueTokenOut2 = testHelper.createMint(6);

                await program.makeDualRedemptionOffer({
                    startTime,
                    endTime,
                    price1,
                    price2,
                    ratioBasisPoints: 8000,
                    feeBasisPoints: 0,
                    tokenInMint: uniqueTokenIn,
                    tokenOutMint1: uniqueTokenOut1,
                    tokenOutMint2: uniqueTokenOut2
                });
            } catch (error) {
                throw error;
            }
        }

        // Verify array is full
        dualRedemptionOfferAccount = await program.getDualRedemptionOfferAccount();
        const activeOffers = dualRedemptionOfferAccount.offers.filter(offer => offer.offerId.toNumber() > 0).length;
        expect(activeOffers).toBe(MAX_DUAL_REDEMPTION_OFFERS);

        // when - try to make one more offer (should fail)
        const finalTokenIn = testHelper.createMint(9);
        const finalTokenOut1 = testHelper.createMint(9);
        const finalTokenOut2 = testHelper.createMint(6);

        await expect(
            program.makeDualRedemptionOffer({
                startTime,
                endTime,
                price1,
                price2,
                ratioBasisPoints: 8000,
                feeBasisPoints: 0,
                tokenInMint: finalTokenIn,
                tokenOutMint1: finalTokenOut1,
                tokenOutMint2: finalTokenOut2
            })
        ).rejects.toThrow("Dual redemption offer account is full");
    });
});