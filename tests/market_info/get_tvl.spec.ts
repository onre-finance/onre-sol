import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";

describe("Get TVL", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);
    });

    describe("SPL Token tests", () => {
        let tokenInMint: PublicKey;
        let tokenOutMint: PublicKey;

        let offerId: number;

        beforeEach(async () => {
            // Create mints with different decimals to test precision handling
            tokenInMint = testHelper.createMint(6); // USDC-like (6 decimals)
            tokenOutMint = testHelper.createMint(9); // ONyc-like (9 decimals)

            // Initialize program and offers
            await program.initialize();
            await program.initializeOffers();

            // Create an offer
            await program.makeOffer({
                tokenInMint,
                tokenOutMint
            });

            const offerAccount = await program.getOfferAccount();
            const offer = offerAccount.offers.find(o => o.offerId.toNumber() !== 0);
            offerId = offer.offerId.toNumber();
        });

        describe("Basic Functionality Tests", () => {
            it("Should successfully get TVL for offer with active vector", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Add vector with base price and APR
                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9, // 1.0 with 9 decimals
                    apr: 36_500, // 3.65% APR (scaled by 1M)
                    priceFixDuration: 86400 // 1 day
                });

                const tvl = await program.getTVL({
                    offerId,
                    tokenOutMint
                });

                // price * supply
                expect(tvl.toString()).toBe("1000099998999900000");
            });

            it("Should be read-only instruction (no state changes)", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Add vector
                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 36_500,
                    priceFixDuration: 86400
                });

                // Get offer state before
                const offerBefore = await program.getOffer(offerId);
                const mintInfoBefore = await testHelper.getMintInfo(tokenOutMint);

                // Call getTVL - should execute successfully
                await program.getTVL({
                    offerId,
                    tokenOutMint
                });

                // Get offer state after
                const offerAfter = await program.getOffer(offerId);
                const mintInfoAfter = await testHelper.getMintInfo(tokenOutMint);

                // Should be identical (no state changes)
                expect(offerAfter).toEqual(offerBefore);
                expect(mintInfoAfter.supply).toBe(mintInfoBefore.supply);
            });
        });

        describe("TVL Calculation Tests", () => {
            it("Should calculate TVL correctly with different prices", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Add vector with different base price
                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 2e9, // 2.0 with 9 decimals
                    apr: 36_500, // 3.65% APR for fixed price
                    priceFixDuration: 86400
                });

                // Should execute successfully with different prices
                const tvl = await program.getTVL({
                    offerId,
                    tokenOutMint
                });

                expect(tvl.toString()).toBe("2000199997999800000");
            });

            it("Should calculate TVL after time advancement and price change", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Add vector with APR
                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 36_500, // 3.65% APR
                    priceFixDuration: 86400 // 1 day intervals
                });

                // Advance time by 1 day (should be in 2nd price interval)
                await testHelper.advanceClockBy(86401);

                // Should execute successfully after time advancement
                const tvl = await program.getTVL({
                    offerId,
                    tokenOutMint
                });

                expect(tvl.toString()).toBe("1000199998999800000");
            });

            it("Should handle 0 APR values correctly", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Test with 0% APR (should maintain base price)
                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 3e9, // 3.0 with 9 decimals
                    apr: 0, // 0% APR
                    priceFixDuration: 86400
                });

                // Should execute successfully with 0 APR
                const tvl = await program.getTVL({
                    offerId,
                    tokenOutMint
                });

                expect(tvl.toString()).toBe("2999999997000000000");
            });
        });

        describe("Error Condition Tests", () => {
            it("Should fail with non-existent offer ID", async () => {
                const nonExistentOfferId = 999;

                await expect(program.getTVL({
                    offerId: nonExistentOfferId,
                    tokenOutMint
                }))
                    .rejects.toThrow("Offer not found");
            });

            it("Should fail with invalid offer ID (0)", async () => {
                await expect(program.getTVL({
                    offerId: 0,
                    tokenOutMint
                }))
                    .rejects.toThrow("Offer not found");
            });

            it("Should fail when offer has no active vectors", async () => {
                // Don't add any vectors to the offer
                await expect(program.getTVL({
                    offerId,
                    tokenOutMint
                }))
                    .rejects.toThrow("No active vector");
            });

            it("Should fail when no vector is active at current time", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Add vector that starts in the future
                await program.addOfferVector({
                    offerId,
                    startTime: currentTime + 86400, // starts tomorrow
                    startPrice: 1e9,
                    apr: 36_500,
                    priceFixDuration: 86400
                });

                await expect(program.getTVL({
                    offerId,
                    tokenOutMint
                }))
                    .rejects.toThrow("No active vector");
            });

            it("Should fail with wrong token_out_mint", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Add vector
                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 36_500,
                    priceFixDuration: 86400
                });

                // Create a different mint
                const wrongMint = testHelper.createMint(9);

                await expect(program.getTVL({
                    offerId,
                    tokenOutMint: wrongMint
                }))
                    .rejects.toThrow("A has one constraint was violated");
            });
        });

        describe("Edge Case Tests", () => {
            it("Should handle multiple vectors and use most recent active one", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Add first vector (older)
                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 36_500,
                    priceFixDuration: 3600
                });

                // Advance time to make the first vector active
                await testHelper.advanceClockBy(1800); // 30 minutes

                // Add second vector (more recent) with different price
                const newCurrentTime = await testHelper.getCurrentClockTime();
                await program.addOfferVector({
                    offerId,
                    startTime: newCurrentTime,
                    startPrice: 5e9, // 5.0 with 9 decimals
                    apr: 0, // 0% APR for fixed price
                    priceFixDuration: 1800
                });

                // Should execute successfully and use the most recent vector
                await program.getTVL({
                    offerId,
                    tokenOutMint
                });
            });

            it("Should handle very large token supplies", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Create a mint with maximum supply used in test helper
                const largeMint = testHelper.createMint(9);

                // Create new offer with the large mint
                await program.makeOffer({
                    tokenInMint,
                    tokenOutMint: largeMint
                });

                const offerAccount = await program.getOfferAccount();
                const offers = offerAccount.offers.filter(o => o.offerId.toNumber() !== 0);
                const newOfferId = offers[offers.length - 1].offerId.toNumber();

                // Add vector
                await program.addOfferVector({
                    offerId: newOfferId,
                    startTime: currentTime,
                    startPrice: 1e9, // 1.0 with 9 decimals
                    apr: 0,
                    priceFixDuration: 86400
                });

                // Should execute successfully with large token supplies
                await program.getTVL({
                    offerId: newOfferId,
                    tokenOutMint: largeMint
                });
            });
        });
    });

    describe("Token2022 tests", () => {
        let tokenInMint: PublicKey;
        let tokenOutMint: PublicKey;

        let offerId: number;

        beforeEach(async () => {
            // Create mints with different decimals to test precision handling
            tokenInMint = testHelper.createMint2022(6); // USDC-like (6 decimals)
            tokenOutMint = testHelper.createMint2022(9); // ONyc-like (9 decimals)

            // Initialize program and offers
            await program.initialize();
            await program.initializeOffers();

            // Create an offer
            await program.makeOffer({
                tokenInMint,
                tokenOutMint,
                tokenInProgram: TOKEN_2022_PROGRAM_ID
            });

            const offerAccount = await program.getOfferAccount();
            const offer = offerAccount.offers.find(o => o.offerId.toNumber() !== 0);
            offerId = offer.offerId.toNumber();
        });

        it("Should calculate TVL correctly with different prices", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector with different base price
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 2e9, // 2.0 with 9 decimals
                apr: 36_500, // 3.65% APR for fixed price
                priceFixDuration: 86400
            });

            // Should execute successfully with different prices
            const tvl = await program.getTVL({
                offerId,
                tokenOutMint,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            expect(tvl.toString()).toBe("2000199997999800000");
        });
    });
});