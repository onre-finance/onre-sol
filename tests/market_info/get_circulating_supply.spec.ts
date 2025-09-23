import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";
import { getAssociatedTokenAddressSync, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";

describe("Get Circulating Supply", () => {
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
            it("Should successfully get circulating supply for offer", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Add vector - not needed for circulating supply calculation but required for offer
                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9, // 1.0 with 9 decimals
                    apr: 36_500, // 3.65% APR (scaled by 1M)
                    priceFixDuration: 86400 // 1 day
                });

                const circulatingSupply = await program.getCirculatingSupply({
                    offerId,
                    tokenOutMint
                });

                // Should equal total supply since vault has 0 tokens initially
                const mintInfo = await testHelper.getMintInfo(tokenOutMint);
                expect(circulatingSupply.toString()).toBe(mintInfo.supply.toString());
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

                // Call getCirculatingSupply - should execute successfully
                await program.getCirculatingSupply({
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

        describe("Circulating Supply Calculation Tests", () => {
            it("Should handle circulating supply calculation with vault logic", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                await program.initializeVaultAuthority();

                // Add vector (not required for circulating supply but good practice)
                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 36_500,
                    priceFixDuration: 86400
                });

                // Boss deposits 10 token_out tokens to vault
                const bossTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, testHelper.getBoss(), BigInt(10e9));
                await program.offerVaultDeposit({
                    amount: 10e9,
                    tokenMint: tokenOutMint
                });

                const vaultTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, program.pdas.offerVaultAuthorityPda, true);
                let vaultTokenOutAccountBalance = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);
                expect(vaultTokenOutAccountBalance).toBe(BigInt(10e9));

                // Check circulating supply with non-zero vault
                let circulatingSupply = await program.getCirculatingSupply({
                    offerId,
                    tokenOutMint
                });

                const mintInfo = await testHelper.getMintInfo(tokenOutMint);
                expect(circulatingSupply.toString()).toBe((mintInfo.supply - vaultTokenOutAccountBalance).toString());

                // Initialize token accounts
                testHelper.createTokenAccount(tokenInMint, testHelper.getBoss(), BigInt(0));
                const user = testHelper.createUserAccount();
                testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(10e6));

                // Check circulating supply after vault amount changes
                await program.takeOffer({
                    offerId,
                    tokenInAmount: 1.0001e6, // Should receive 1 token out
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                vaultTokenOutAccountBalance = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);
                expect(vaultTokenOutAccountBalance).toBe(BigInt(9e9));

                // Check circulating supply with non-zero vault
                circulatingSupply = await program.getCirculatingSupply({
                    offerId,
                    tokenOutMint
                });

                expect(circulatingSupply.toString()).toBe((mintInfo.supply - vaultTokenOutAccountBalance).toString());
            });

            it("Should handle zero vault balance correctly", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Add vector
                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 0, // 0% APR for fixed price
                    priceFixDuration: 86400
                });

                const circulatingSupply = await program.getCirculatingSupply({
                    offerId,
                    tokenOutMint
                });

                const mintInfo = await testHelper.getMintInfo(tokenOutMint);

                // Should equal total supply when vault is empty
                expect(circulatingSupply.toString()).toBe(mintInfo.supply.toString());
            });
        });

        describe("Error Condition Tests", () => {
            it("Should fail with non-existent offer ID", async () => {
                const nonExistentOfferId = 999;

                await expect(program.getCirculatingSupply({
                    offerId: nonExistentOfferId,
                    tokenOutMint
                }))
                    .rejects.toThrow("Offer not found");
            });

            it("Should fail with invalid offer ID (0)", async () => {
                await expect(program.getCirculatingSupply({
                    offerId: 0,
                    tokenOutMint
                }))
                    .rejects.toThrow("Offer not found");
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

                await expect(program.getCirculatingSupply({
                    offerId,
                    tokenOutMint: wrongMint
                }))
                    .rejects.toThrow("A has one constraint was violated");
            });
        });

        describe("Edge Case Tests", () => {
            it("Should handle large token supplies", async () => {
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
                const circulatingSupply = await program.getCirculatingSupply({
                    offerId: newOfferId,
                    tokenOutMint: largeMint
                });

                const mintInfo = await testHelper.getMintInfo(largeMint);
                expect(circulatingSupply.toString()).toBe(mintInfo.supply.toString());
            });

            it("Should work without active vectors since it doesn't depend on pricing", async () => {
                // Don't add any vectors to the offer - circulating supply should still work
                const circulatingSupply = await program.getCirculatingSupply({
                    offerId,
                    tokenOutMint
                });

                const mintInfo = await testHelper.getMintInfo(tokenOutMint);
                expect(circulatingSupply.toString()).toBe(mintInfo.supply.toString());
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

        it("Should calculate circulating supply correctly for Token2022", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 2e9, // 2.0 with 9 decimals
                apr: 36_500, // 3.65% APR for fixed price
                priceFixDuration: 86400
            });

            // Should execute successfully with Token2022
            const circulatingSupply = await program.getCirculatingSupply({
                offerId,
                tokenOutMint,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            const mintInfo = await testHelper.getMintInfo(tokenOutMint);
            expect(circulatingSupply.toString()).toBe(mintInfo.supply.toString());
        });
    });
});