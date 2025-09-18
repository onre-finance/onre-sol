import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Delete Buy Offer Vector", () => {
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

    it("Should delete an existing vector from a buy offer", async () => {
        const offerId = 1;

        // Create a buy offer
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add a vector to the offer
        await program.addBuyOfferVector({
            offerId,
            startTime: currentTime + 1000,
            startPrice: 1000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        // Verify vector was added
        let offer = await program.getOffer(offerId);
        let activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);
        expect(activeVectors.length).toBe(1);
        expect(activeVectors[0].vectorId.toNumber()).toBe(1);

        // Delete the vector
        await program.deleteBuyOfferVector({
            offerId,
            vectorId: 1
        });

        // Verify vector was deleted
        offer = await program.getOffer(offerId);
        activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);
        expect(activeVectors.length).toBe(0);
    });

    it("Should fail when offer_id is zero", async () => {
        await expect(
            program.deleteBuyOfferVector({
                offerId: 0,
                vectorId: 1
            })
        ).rejects.toThrow("Buy offer with the specified ID was not found");
    });

    it("Should fail when vector_id is zero", async () => {
        const offerId = 1;

        // Create a buy offer
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        await expect(
            program.deleteBuyOfferVector({
                offerId,
                vectorId: 0
            })
        ).rejects.toThrow("Vector with the specified ID was not found in the offer");
    });

    it("Should fail when offer doesn't exist", async () => {
        const nonExistentOfferId = 999;

        await expect(
            program.deleteBuyOfferVector({
                offerId: nonExistentOfferId,
                vectorId: 1
            })
        ).rejects.toThrow("Offer not found");
    });

    it("Should fail when vector doesn't exist in the offer", async () => {
        const offerId = 1;

        // Create a buy offer
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        await expect(
            program.deleteBuyOfferVector({
                offerId,
                vectorId: 999
            })
        ).rejects.toThrow("Vector with the specified ID was not found in the offer");
    });

    it("Should delete specific vector while keeping others", async () => {
        const offerId = 1;

        // Create a buy offer
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add three vectors
        for (let i = 1; i <= 3; i++) {
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + (i * 1000),
                startPrice: i * 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });
        }

        // Verify all vectors were added
        let offer = await program.getOffer(offerId);
        let activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);
        expect(activeVectors.length).toBe(3);

        // Delete the middle vector (vector_id = 2)
        await program.deleteBuyOfferVector({
            offerId,
            vectorId: 2
        });

        // Verify only vector 2 was deleted
        offer = await program.getOffer(offerId);
        activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

        expect(activeVectors.length).toBe(2);
        const vectorIds = activeVectors.map(v => v.vectorId.toNumber()).sort();
        expect(vectorIds).toEqual([1, 3]);

        // Verify prices of remaining vectors
        const vectorPrices = activeVectors.map(v => v.basePrice.toNumber()).sort();
        expect(vectorPrices).toContain(1000000); // Vector 1
        expect(vectorPrices).toContain(3000000); // Vector 3
        expect(vectorPrices).not.toContain(2000000); // Vector 2 deleted
    });

    it("Should reject when called by non-boss", async () => {
        const offerId = 1;

        // Create a buy offer
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add a vector
        await program.addBuyOfferVector({
            offerId,
            startTime: currentTime + 1000,
            startPrice: 1000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        const notBoss = testHelper.createUserAccount();

        await expect(
            program.deleteBuyOfferVector({
                offerId,
                vectorId: 1,
                signer: notBoss
            })
        ).rejects.toThrow(); // Should fail due to boss constraint
    });

    describe("Previously Active Vector Validation", () => {
        it("Should prevent deletion of previously active vector", async () => {
            const offerId = 1;

            // Create a buy offer
            await program.makeBuyOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add only 2 vectors in the future to keep it simple
            // Vector 1: will become previous active
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 100, // 100 seconds in future
                startPrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            // Vector 2: will become currently active
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 200, // 200 seconds in future
                startPrice: 2000000,
                apr: 7500,
                priceFixDuration: 3600
            });

            // Advance time to make vector 2 active (and vector 1 previous active)
            await testHelper.advanceClockBy(250); // Move 250 seconds forward (past both vectors)

            // Verify we have 2 vectors
            const offer = await program.getOffer(offerId);
            const activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(2);

            // Now try to delete vector 1 (previous active) - should fail
            await expect(
                program.deleteBuyOfferVector({
                    offerId,
                    vectorId: 1
                })
            ).rejects.toThrow("Cannot delete previously active vector");
        });

        it("Should allow deletion of current active vector", async () => {
            const offerId = 1;

            // Create a buy offer
            await program.makeBuyOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add vectors in the future
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 10, // 10 seconds in future
                startPrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 20, // 20 seconds in future
                startPrice: 2000000,
                apr: 7500,
                priceFixDuration: 3600
            });

            // Advance time to make vector 2 active
            await testHelper.advanceClockBy(25);

            // Delete the current active vector (vector_id = 2) - should succeed
            await program.deleteBuyOfferVector({
                offerId,
                vectorId: 2
            });

            // Verify deletion succeeded
            const offer = await program.getOffer(offerId);
            const activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(1);
            expect(activeVectors[0].vectorId.toNumber()).toBe(1);
        });

        it("Should allow deletion of future vector", async () => {
            const offerId = 1;

            // Create a buy offer
            await program.makeBuyOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add vectors in the future
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 10, // 10 seconds in future
                startPrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 20, // 20 seconds in future
                startPrice: 2000000,
                apr: 7500,
                priceFixDuration: 3600
            });

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 30, // 30 seconds in future
                startPrice: 3000000,
                apr: 10000,
                priceFixDuration: 3600
            });

            // Advance time to make vector 2 active (vector 3 remains future)
            await testHelper.advanceClockBy(25);

            // Delete the future vector (vector_id = 3) - should succeed
            await program.deleteBuyOfferVector({
                offerId,
                vectorId: 3
            });

            // Verify deletion succeeded
            const offer = await program.getOffer(offerId);
            const activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(2);
            const vectorIds = activeVectors.map(v => v.vectorId.toNumber()).sort();
            expect(vectorIds).toEqual([1, 2]);
        });

        it("Should allow deletion when there's only one vector (no previous vector)", async () => {
            const offerId = 1;

            // Create a buy offer
            await program.makeBuyOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add only one vector in the future
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 10, // 10 seconds in future
                startPrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            // Advance time to make it active
            await testHelper.advanceClockBy(15);

            // Delete the only vector - should succeed (no previous vector exists)
            await program.deleteBuyOfferVector({
                offerId,
                vectorId: 1
            });

            // Verify deletion succeeded
            const offer = await program.getOffer(offerId);
            const activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(0);
        });

        it("Should allow deletion of past vectors that are not previously active", async () => {
            const offerId = 1;

            // Create a buy offer
            await program.makeBuyOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add 4 vectors in the future to create a sequence
            // Vector 1: will be old past vector (deletable)
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 100, // 100 seconds in future
                startPrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            // Vector 2: will be old past vector (deletable)
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 200, // 200 seconds in future
                startPrice: 2000000,
                apr: 6000,
                priceFixDuration: 3600
            });

            // Vector 3: will become previously active (protected)
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 300, // 300 seconds in future
                startPrice: 3000000,
                apr: 7000,
                priceFixDuration: 3600
            });

            // Vector 4: will become currently active
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 400, // 400 seconds in future
                startPrice: 4000000,
                apr: 8000,
                priceFixDuration: 3600
            });

            // Advance time to make vector 4 active (vector 3 becomes previous active)
            await testHelper.advanceClockBy(450);

            // Verify we have all 4 vectors
            let offer = await program.getOffer(offerId);
            let activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(4);

            // Try to delete vector 1 (old past vector, not previously active) - should succeed
            await program.deleteBuyOfferVector({
                offerId,
                vectorId: 1
            });

            // Verify vector 1 was deleted
            offer = await program.getOffer(offerId);
            activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(3);
            expect(activeVectors.map(v => v.vectorId.toNumber()).sort()).toEqual([2, 3, 4]);

            // Try to delete vector 2 (another old past vector, not previously active) - should succeed
            await program.deleteBuyOfferVector({
                offerId,
                vectorId: 2
            });

            // Verify vector 2 was deleted
            offer = await program.getOffer(offerId);
            activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(2);
            expect(activeVectors.map(v => v.vectorId.toNumber()).sort()).toEqual([3, 4]);

            // Try to delete vector 3 (previously active) - should fail
            await expect(
                program.deleteBuyOfferVector({
                    offerId,
                    vectorId: 3
                })
            ).rejects.toThrow("Cannot delete previously active vector");
        });

        it("Should allow deletion when all vectors are in the future (no active vector)", async () => {
            const offerId = 1;

            // Create a buy offer
            await program.makeBuyOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add vectors that are all in the future
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 100, // 100 seconds in future
                startPrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 200, // 200 seconds in future
                startPrice: 2000000,
                apr: 7500,
                priceFixDuration: 3600
            });

            // Delete the first future vector - should succeed (no active vector means no previous vector)
            await program.deleteBuyOfferVector({
                offerId,
                vectorId: 1
            });

            // Verify deletion succeeded
            const offer = await program.getOffer(offerId);
            const activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(1);
            expect(activeVectors[0].vectorId.toNumber()).toBe(2);
        });
    });
});