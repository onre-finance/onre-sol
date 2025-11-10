import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Delete Offer Vector", () => {
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
        await program.initialize({ onycMint: tokenOutMint });
    });

    it("Should delete an existing vector from an offer", async () => {
        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add a vector to the offer
        await program.addOfferVector({
            tokenInMint,
            tokenOutMint,
            baseTime: currentTime + 1000,
            basePrice: 1000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        // Verify vector was added
        let offer = await program.getOffer(tokenInMint, tokenOutMint);
        let activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(1);
        expect(activeVectors[0].startTime.toNumber()).toBe(currentTime + 1000);

        // Delete the vector using its startTime
        await program.deleteOfferVector(
            tokenInMint,
            tokenOutMint,
            currentTime + 1000
        );

        // Verify vector was deleted
        offer = await program.getOffer(tokenInMint, tokenOutMint);
        activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(0);
    });

    it("Should fail with incorrect token mints", async () => {
        await expect(
            program.deleteOfferVector(
                tokenInMint,
                testHelper.createMint(9),
                1000
            )
        ).rejects.toThrow("AnchorError caused by account: offer");

        await expect(
            program.deleteOfferVector(
                testHelper.createMint(9),
                tokenOutMint,
                1000
            )
        ).rejects.toThrow("AnchorError caused by account: offer");
    });

    it("Should fail when vector start_time is zero", async () => {
        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        await expect(
            program.deleteOfferVector(
                tokenInMint,
                tokenOutMint,
                0
            )
        ).rejects.toThrow("Vector not found");
    });

    it("Should fail when vector doesn't exist in the offer", async () => {
        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        await expect(
            program.deleteOfferVector(
                tokenInMint,
                tokenOutMint,
                999
            )
        ).rejects.toThrow("Vector not found");
    });

    it("Should delete specific vector while keeping others", async () => {
        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add three vectors
        for (let i = 1; i <= 3; i++) {
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + (i * 1000),
                basePrice: i * 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });
        }

        // Verify all vectors were added
        let offer = await program.getOffer(tokenInMint, tokenOutMint);
        let activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);
        expect(activeVectors.length).toBe(3);

        // Delete the middle vector (with start_time = currentTime + 2000)
        await program.deleteOfferVector(
            tokenInMint,
            tokenOutMint,
            currentTime + 2000
        );

        // Verify only the middle vector was deleted
        offer = await program.getOffer(tokenInMint, tokenOutMint);
        activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);

        expect(activeVectors.length).toBe(2);
        const startTimes = activeVectors.map(v => v.startTime.toNumber()).sort();
        expect(startTimes).toEqual([currentTime + 1000, currentTime + 3000]);

        // Verify prices of remaining vectors
        const vectorPrices = activeVectors.map(v => v.basePrice.toNumber()).sort();
        expect(vectorPrices).toContain(1000000); // Vector 1
        expect(vectorPrices).toContain(3000000); // Vector 3
        expect(vectorPrices).not.toContain(2000000); // Vector 2 deleted
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
            program.deleteOfferVector(
                tokenInMint,
                tokenOutMint,
                1,
                notBoss
            )
        ).rejects.toThrow(); // Should fail due to boss constraint
    });

    describe("Previously Active Vector Validation", () => {
        it("Should prevent deletion of previously active vector", async () => {
            // Create an offer
            await program.makeOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add only 2 vectors in the future to keep it simple
            // Vector 1: will become previous active
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 100, // 100 seconds in future
                basePrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            // Vector 2: will become currently active
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 200, // 200 seconds in future
                basePrice: 2000000,
                apr: 7500,
                priceFixDuration: 3600
            });

            // Advance time to make vector 2 active (and vector 1 previous active)
            await testHelper.advanceClockBy(250); // Move 250 seconds forward (past both vectors)

            // Verify we have 2 vectors
            const offer = await program.getOffer(tokenInMint, tokenOutMint);
            const activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);

            expect(activeVectors.length).toBe(2);

            // Now try to delete vector 1 (previous active) - should fail
            await expect(
                program.deleteOfferVector(
                    tokenInMint,
                    tokenOutMint,
                    currentTime + 100  // This is the start_time of the first vector
                )
            ).rejects.toThrow("Cannot delete previously active vector");
        });

        it("Should allow deletion of current active vector", async () => {
            // Create an offer
            await program.makeOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add vectors in the future
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 10, // 10 seconds in future
                basePrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 20, // 20 seconds in future
                basePrice: 2000000,
                apr: 7500,
                priceFixDuration: 3600
            });

            // Advance time to make vector 2 active
            await testHelper.advanceClockBy(25);

            // Delete the current active vector (startTime = currentTime + 20) - should succeed
            await program.deleteOfferVector(
                tokenInMint,
                tokenOutMint,
                currentTime + 20
            );

            // Verify deletion succeeded
            const offer = await program.getOffer(tokenInMint, tokenOutMint);
            const activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);

            expect(activeVectors.length).toBe(1);
            expect(activeVectors[0].startTime.toNumber()).toBe(currentTime + 10);
        });

        it("Should allow deletion of future vector", async () => {
            // Create an offer
            await program.makeOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add vectors in the future
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 10, // 10 seconds in future
                basePrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 20, // 20 seconds in future
                basePrice: 2000000,
                apr: 7500,
                priceFixDuration: 3600
            });

            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 30, // 30 seconds in future
                basePrice: 3000000,
                apr: 10000,
                priceFixDuration: 3600
            });

            // Advance time to make vector 2 active (vector 3 remains future)
            await testHelper.advanceClockBy(25);

            // Delete the future vector (startTime = currentTime + 30) - should succeed
            await program.deleteOfferVector(
                tokenInMint,
                tokenOutMint,
                currentTime + 30
            );

            // Verify deletion succeeded
            const offer = await program.getOffer(tokenInMint, tokenOutMint);
            const activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);

            expect(activeVectors.length).toBe(2);
            const startTimes = activeVectors.map(v => v.startTime.toNumber()).sort();
            expect(startTimes).toEqual([currentTime + 10, currentTime + 20]);
        });

        it("Should allow deletion when there's only one vector (no previous vector)", async () => {
            // Create an offer
            await program.makeOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add only one vector in the future
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 10, // 10 seconds in future
                basePrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            // Advance time to make it active
            await testHelper.advanceClockBy(15);

            // Delete the only vector - should succeed (no previous vector exists)
            await program.deleteOfferVector(
                tokenInMint,
                tokenOutMint,
                currentTime + 10
            );

            // Verify deletion succeeded
            const offer = await program.getOffer(tokenInMint, tokenOutMint);
            const activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);

            expect(activeVectors.length).toBe(0);
        });

        it("Should allow deletion of past vectors that are not previously active", async () => {
            // Create an offer
            await program.makeOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add 4 vectors in the future to create a sequence
            // Vector 1: will be old past vector (deletable)
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 100, // 100 seconds in future
                basePrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            // Vector 2: will be old past vector (deletable)
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 200, // 200 seconds in future
                basePrice: 2000000,
                apr: 6000,
                priceFixDuration: 3600
            });

            // Vector 3: will become previously active (protected)
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 300, // 300 seconds in future
                basePrice: 3000000,
                apr: 7000,
                priceFixDuration: 3600
            });

            // Vector 4: will become currently active
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 400, // 400 seconds in future
                basePrice: 4000000,
                apr: 8000,
                priceFixDuration: 3600
            });

            // Advance time to make vector 4 active (vector 3 becomes previous active)
            await testHelper.advanceClockBy(450);

            // Verify we have all 4 vectors
            let offer = await program.getOffer(tokenInMint, tokenOutMint);
            let activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);

            expect(activeVectors.length).toBe(4);

            // Try to delete vector 1 (old past vector, not previously active) - should succeed
            await program.deleteOfferVector(
                tokenInMint,
                tokenOutMint,
                currentTime + 100
            );

            // Verify vector 1 was deleted
            offer = await program.getOffer(tokenInMint, tokenOutMint);
            activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);

            expect(activeVectors.length).toBe(3);
            expect(activeVectors.map(v => v.startTime.toNumber()).sort()).toEqual([currentTime + 200, currentTime + 300, currentTime + 400]);

            // Try to delete vector 2 (another old past vector, not previously active) - should succeed
            await program.deleteOfferVector(
                tokenInMint,
                tokenOutMint,
                currentTime + 200
            );

            // Verify vector 2 was deleted
            offer = await program.getOffer(tokenInMint, tokenOutMint);
            activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);

            expect(activeVectors.length).toBe(2);
            expect(activeVectors.map(v => v.startTime.toNumber()).sort()).toEqual([currentTime + 300, currentTime + 400]);

            // Try to delete vector 3 (previously active) - should fail
            await expect(
                program.deleteOfferVector(
                    tokenInMint,
                    tokenOutMint,
                    currentTime + 300
                )
            ).rejects.toThrow("Cannot delete previously active vector");
        });

        it("Should allow deletion when all vectors are in the future (no active vector)", async () => {
            // Create an offer
            await program.makeOffer({
                tokenInMint,
                tokenOutMint
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add vectors that are all in the future
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 100, // 100 seconds in future
                basePrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            });

            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime + 200, // 200 seconds in future
                basePrice: 2000000,
                apr: 7500,
                priceFixDuration: 3600
            });

            // Delete the first future vector - should succeed (no active vector means no previous vector)
            await program.deleteOfferVector(
                tokenInMint,
                tokenOutMint,
                currentTime + 100
            );

            // Verify deletion succeeded
            const offer = await program.getOffer(tokenInMint, tokenOutMint);
            const activeVectors = offer.vectors.filter(v => v.startTime.toNumber() !== 0);

            expect(activeVectors.length).toBe(1);
            expect(activeVectors[0].startTime.toNumber()).toBe(currentTime + 200);
        });
    });
});