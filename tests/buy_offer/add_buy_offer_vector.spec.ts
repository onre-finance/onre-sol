import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { BN } from "@coral-xyz/anchor";
import { OnreProgram } from "../onre_program.ts";

const MAX_SEGMENTS = 10;

describe("Add Buy Offer Vector", () => {
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

    it("Should create a buy offer and add a time vector", async () => {
        // First create a buy offer using testHelper
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        // Get the first offer (auto-generated ID)
        const offerId = 1;

        // Add a time vector to the offer
        const currentTime = await testHelper.getCurrentClockTime();
        const startTime = currentTime + 3600; // 1 hour in future
        const startPrice = 1000000; // 1 token
        const apr = 5000;    // 50% APR (5000/10000)
        const priceFixDuration = 3600; // 1 hour

        await program.addBuyOfferVector({
            offerId,
            startTime,
            startPrice,
            apr,
            priceFixDuration
        });

        // Verify the time vector was added
        const updatedOffer = await program.getOffer(offerId);

        const vector = updatedOffer.vectors[0];
        expect(vector.vectorId.toString()).toBe("1");
        expect(vector.baseTime.toString()).toBe(startTime.toString());
        expect(vector.startTime.toString()).toBe(startTime.toString()); // start_time should equal base_time when base_time is in future
        expect(vector.basePrice.toString()).toBe(startPrice.toString());
        expect(vector.apr.toString()).toBe(apr.toString());
        expect(vector.priceFixDuration.toString()).toBe(priceFixDuration.toString());
    });

    it("Should calculate start_time as current time when base_time is in the past", async () => {
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const offerId = 1;
        const currentTime = await testHelper.getCurrentClockTime();

        await program.addBuyOfferVector({
            offerId,
            startTime: currentTime - 3600, // 1 hour ago,
            startPrice: 1000000,
            apr: 250000, // 25% APR
            priceFixDuration: 1000
        });

        const updatedOffer = await program.getOffer(offerId);
        const vector = updatedOffer.vectors[0];

        expect(vector.baseTime.toString()).toBe((currentTime - 3600).toString());
        expect(vector.startTime.toNumber()).toBe(currentTime);
    });

    it("Should auto-increment vector IDs correctly", async () => {
        const offerId = 1;
        // Create buy offer
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add first vector
        await program.addBuyOfferVector({
            offerId,
            startTime: currentTime + 1000,
            startPrice: 1000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        // Add second vector (with later base_time)
        await program.addBuyOfferVector({
            offerId,
            startTime: currentTime + 3000,
            startPrice: 2000000,
            apr: 7500,
            priceFixDuration: 1800
        });

        // Add third vector
        await program.addBuyOfferVector({
            offerId,
            startTime: currentTime + 5000,
            startPrice: 3000000,
            apr: 1000,
            priceFixDuration: 900
        });

        // Verify vectors have correct auto-incremented IDs
        const offer = await program.getOffer(offerId);

        expect(offer.vectors[0].vectorId.toString()).toBe("1");
        expect(offer.vectors[1].vectorId.toString()).toBe("2");
        expect(offer.vectors[2].vectorId.toString()).toBe("3");
    });

    it("Should reject invalid parameters", async () => {
        const currentTime = await testHelper.getCurrentClockTime();
        const offerId = 1;
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        await expect(
            program.addBuyOfferVector({
                offerId: 0, // Invalid: zero offer_id
                startTime: currentTime + 1000,
                startPrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            })
        ).rejects.toThrow("Invalid input: values cannot be zero");

        await expect(
            program.addBuyOfferVector({
                offerId,
                startTime: 0, // Invalid: zero base_time
                startPrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            })
        ).rejects.toThrow("Invalid input: values cannot be zero");

        await expect(
            program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 0, // Invalid: zero base_price
                apr: 5000,
                priceFixDuration: 3600
            })
        ).rejects.toThrow("Invalid input: values cannot be zero");

        await expect(
            program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1000000,
                apr: 5000,
                priceFixDuration: 0 // Invalid: zero price_fix_duration
            })
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should allow zero apr", async () => {
        const offerId = 1;
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        await program
            .addBuyOfferVector({
                offerId,
                startTime: currentTime + 1000,
                startPrice: 1000000,
                apr: 0, // Zero APR
                priceFixDuration: 3600
            });

        // Verify vector was added correctly
        const offer = await program.getOffer(offerId);
        const vector = offer.vectors[0];

        expect(vector.apr.toString()).toBe("0");
    });

    it("Should reject start_time before latest existing vector start_time", async () => {
        const offerId = 1;
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add first vector
        await program.addBuyOfferVector({
            offerId,
            startTime: currentTime + 2000,
            startPrice: 1000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        // Try to add vector with earlier base_time (should fail)
        await expect(
            program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 1000, // Invalid: before previous start_time
                startPrice: 2000000,
                apr: 7500,
                priceFixDuration: 1800
            })
        ).rejects.toThrow("Invalid time range: base_time must be after the latest existing vector");
    });

    it("Should reject start_time equal to latest existing vector start_time", async () => {
        const offerId = 1;
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();
        const startTime = currentTime + 2000;

        // Add first vector
        await program.addBuyOfferVector({
            offerId,
            startTime,
            startPrice: 1000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        // Add vector with same start_time (should fail)
        await expect(program
            .addBuyOfferVector({
                offerId,
                startTime, // Same start_time - should be allowed
                startPrice: 2000000,
                apr: 7500,
                priceFixDuration: 1800
            }))
            .rejects.toThrow("Invalid time range: base_time must be after the latest existing vector.");
    });

    it("Should reject adding vector to non-existent offer", async () => {
        const nonExistentOfferId = 999999;
        const currentTime = await testHelper.getCurrentClockTime();

        await expect(
            program.addBuyOfferVector({
                offerId: nonExistentOfferId,
                startTime: currentTime + 1000,
                startPrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600
            })
        ).rejects.toThrow("Offer not found");
    });

    it("Should reject when offer has maximum vectors", async () => {
        const offerId = 1;
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();
        const vectorTimeOffset = 1000;
        const startPrice = 1000000;
        const apr = 5000;
        const priceFixDuration = 3600;

        // Add maximum number of vectors
        for (let i = 1; i <= MAX_SEGMENTS; i++) {
            const vectorStartTime = currentTime + (i * vectorTimeOffset);

            await program.addBuyOfferVector({
                offerId,
                startTime: vectorStartTime,
                startPrice,
                apr,
                priceFixDuration
            });
        }

        // Try to add one more vector (should fail)
        const vectorStartTime = currentTime + ((MAX_SEGMENTS + 1) * vectorTimeOffset);

        await expect(
            program.addBuyOfferVector({
                offerId,
                startTime: vectorStartTime,
                startPrice,
                apr,
                priceFixDuration
            })
        ).rejects.toThrow("Cannot add more vectors: maximum limit reached");
    });

    it("Should handle large price and apr values correctly", async () => {
        const offerId = new BN(1);
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Use large values
        const largeStartPrice = new BN("999999999999999999"); // Large u64 value
        const largeApr = new BN(999999); // 99.9999% APR (9999/10000)

        await program.program.methods
            .addBuyOfferVector(
                offerId,
                new BN(currentTime + 1000),
                largeStartPrice,
                largeApr,
                new BN(3600)
            )
            .accounts({
                state: program.statePda
            })
            .rpc();

        // Verify the vector was added with large values
        const offer = await program.getOffer(offerId.toNumber());
        const vector = offer.vectors[0];

        expect(vector.basePrice.toString()).toBe(largeStartPrice.toString());
        expect(vector.apr.toString()).toBe(largeApr.toString());
    });

    it("Should handle minimum valid values (1 for all fields)", async () => {
        const offerId = 1;
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        await program.addBuyOfferVector({
            offerId,
            startTime: 1, // Minimum valid start_time
            startPrice: 1, // Minimum valid start_price
            apr: 0, // Minimum valid apr
            priceFixDuration: 1  // Minimum valid price_fix_duration
        });

        // Verify the vector was added
        const offer = await program.getOffer(offerId);
        const vector = offer.vectors[0];

        expect(vector.vectorId.toString()).toBe("1");
        expect(vector.baseTime.toString()).toBe("1");
        // start_time should be current time since base_time=1 is in the past
        const currentTime = await testHelper.getCurrentClockTime();
        const startTime = parseInt(vector.startTime.toString());
        expect(startTime).toBeGreaterThanOrEqual(currentTime);
        expect(vector.basePrice.toString()).toBe("1");
        expect(vector.apr.toString()).toBe("0");
        expect(vector.priceFixDuration.toString()).toBe("1");
    });

    it("Should reject when called by non-boss", async () => {
        const offerId = 1;
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const notBoss = testHelper.createUserAccount();
        const currentTime = await testHelper.getCurrentClockTime();

        await expect(program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 1000,
                startPrice: 1000000,
                apr: 5000,
                priceFixDuration: 3600,
                signer: notBoss
            })
        ).rejects.toThrow(); // Should fail due to boss constraint
    });

    it("Should handle vectors added to multiple different offers", async () => {
        const offer1Id = 1;
        const offer2Id = 2;

        // Create two offers
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

        const currentTime = await testHelper.getCurrentClockTime();

        // Add vectors to both offers
        await program.addBuyOfferVector({
            offerId: offer1Id,
            startTime: currentTime + 1000,
            startPrice: 1000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        await program.addBuyOfferVector({
            offerId: offer2Id,
            startTime: currentTime + 1000,
            startPrice: 3000000,
            apr: 7500,
            priceFixDuration: 1800
        });

        await program.addBuyOfferVector({
            offerId: offer1Id,
            startTime: currentTime + 3000,
            startPrice: 2000000,
            apr: 2500,
            priceFixDuration: 3600
        });

        // Verify each offer has its own vector ID sequence
        const offer1 = await program.getOffer(offer1Id);
        const offer2 = await program.getOffer(offer2Id);

        // Offer 1 should have vectors 1 and 2
        expect(offer1.vectors[0].vectorId.toString()).toBe("1");
        expect(offer1.vectors[1].vectorId.toString()).toBe("2");

        // Offer 2 should have vector 1 (independent sequence)
        expect(offer2.vectors[0].vectorId.toString()).toBe("1");

        // Verify prices are correct for each offer
        expect(offer1.vectors[0].basePrice.toString()).toBe("1000000");
        expect(offer2.vectors[0].basePrice.toString()).toBe("3000000");

        // Verify APRs are correct
        expect(offer1.vectors[0].apr.toString()).toBe("5000"); // 50%
        expect(offer2.vectors[0].apr.toString()).toBe("7500"); // 75%
    });

    it("Should clean old past vectors, keeping only current active and previous active", async () => {
        const offerId = 1;
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });
        const currentTime = await testHelper.getCurrentClockTime();

        // Add 5 vectors: all in the future
        const vectors = [
            { startTime: currentTime + 1000, price: 1000000 },
            { startTime: currentTime + 2000, price: 2000000 },
            { startTime: currentTime + 3000, price: 3000000 },
            { startTime: currentTime + 4000, price: 4000000 },
            { startTime: currentTime + 5000, price: 5000000 }
        ];

        // Add all vectors
        for (let i = 0; i < vectors.length; i++) {
            await program.addBuyOfferVector({
                offerId,
                startTime: vectors[i].startTime,
                startPrice: vectors[i].price,
                apr: 5000, // 0.5% APR
                priceFixDuration: 3600  // 1 hour duration
            });
        }

        // Verify all 5 vectors were added
        let offer = await program.getOffer(offerId);
        let activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);
        expect(activeVectors.length).toBe(5);

        // Time travel so the 4th vector is now active
        await testHelper.advanceClockBy(4500);

        // Add another vector to trigger cleanup
        await program.addBuyOfferVector({
            offerId,
            startTime: currentTime + 6000, // Vector 6 (future)
            startPrice: 6000000,
            apr: 5000,
            priceFixDuration: 3600
        });

        // After cleanup, should only have:
        // - Vector 4 (currently active - most recent start_time <= current_time)
        // - Vector 3 (previously active - closest smaller vector_id to vector 3)
        // - Vector 5 (future vector - should be kept)
        // - Vector 6 (newly added future vector)
        // Vector 1 and 2 should be deleted (oldest past vector)
        offer = await program.getOffer(offerId);
        activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

        // Should have exactly 4 vectors remaining (deleted vector 1 and 2)
        expect(activeVectors.length).toBe(4);

        // Find vectors by their prices to identify them
        const remainingVectorIds = activeVectors.map(v => v.vectorId.toNumber()).sort();
        const remainingPrices = activeVectors.map(v => v.basePrice.toNumber()).sort();

        // Should have vectors 2, 3, 4, 5, and 6
        expect(remainingVectorIds).toEqual([3, 4, 5, 6]);

        // Verify the specific prices are present
        expect(remainingPrices).toContain(3000000); // Vector 3 (previous active)
        expect(remainingPrices).toContain(4000000); // Vector 4 (current active)
        expect(remainingPrices).toContain(5000000); // Vector 5 (future)
        expect(remainingPrices).toContain(6000000); // Vector 6 (newly added)

        // Verify vector 1 and 2 were cleaned up (oldest past vector)
        expect(remainingPrices).not.toContain(1000000); // Vector 1 price
        expect(remainingPrices).not.toContain(2000000); // Vector 2 price
    });
});