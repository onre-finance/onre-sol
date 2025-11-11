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

        beforeEach(async () => {
            // Create mints with different decimals to test precision handling
            tokenInMint = testHelper.createMint(6); // USDC-like (6 decimals)
            tokenOutMint = testHelper.createMint(9); // ONyc-like (9 decimals)

            // Initialize program and offers
            await program.initialize({ onycMint: tokenOutMint });
        });

        describe("Basic Functionality Tests", () => {
            it("Should successfully get circulating supply for offer", async () => {
                const circulatingSupply = await program.getCirculatingSupply({
                    onycMint: tokenOutMint
                });

                // Should equal total supply since vault has 0 tokens initially
                const mintInfo = await testHelper.getMintInfo(tokenOutMint);
                expect(circulatingSupply.toString()).toBe(mintInfo.supply.toString());
            });

            it("Should be read-only instruction (no state changes)", async () => {
                // Get offer state before
                const mintInfoBefore = await testHelper.getMintInfo(tokenOutMint);
                const offerStateBefore = await program.getState();

                // Call getCirculatingSupply - should execute successfully
                await program.getCirculatingSupply({
                    onycMint: tokenOutMint
                });

                // Get offer state after
                const mintInfoAfter = await testHelper.getMintInfo(tokenOutMint);
                const offerStateAfter = await program.getState();

                // Should be identical (no state changes)
                expect(offerStateBefore).toEqual(offerStateAfter);
                expect(mintInfoAfter.supply).toBe(mintInfoBefore.supply);
            });
        });

        describe("Circulating Supply Calculation Tests", () => {
            it("Should handle circulating supply calculation with vault logic", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                // Create an offer
                await program.makeOffer({ tokenInMint, tokenOutMint });

                // Add vector (not required for circulating supply but good practice)
                await program.addOfferVector({
                    tokenInMint,
                    tokenOutMint,
                    baseTime: currentTime,
                    basePrice: 1e9,
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
                let circulatingSupply = await program.getCirculatingSupply({ onycMint: tokenOutMint });

                const mintInfo = await testHelper.getMintInfo(tokenOutMint);
                expect(circulatingSupply.toString()).toBe((mintInfo.supply - vaultTokenOutAccountBalance).toString());

                // Initialize token accounts
                testHelper.createTokenAccount(tokenInMint, testHelper.getBoss(), BigInt(0));
                const user = testHelper.createUserAccount();
                testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(10e6));

                // Check circulating supply after vault amount changes
                await program.takeOffer({
                    tokenInAmount: 1.0001e6, // Should receive 1 token out
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                vaultTokenOutAccountBalance = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);
                expect(vaultTokenOutAccountBalance).toBe(BigInt(9e9));

                // Check circulating supply with non-zero vault
                circulatingSupply = await program.getCirculatingSupply({ onycMint: tokenOutMint });

                expect(circulatingSupply.toString()).toBe((mintInfo.supply - vaultTokenOutAccountBalance).toString());
            });

            it("Should handle zero vault balance correctly", async () => {
                const circulatingSupply = await program.getCirculatingSupply({ onycMint: tokenOutMint });

                const mintInfo = await testHelper.getMintInfo(tokenOutMint);

                // Should equal total supply when vault is empty
                expect(circulatingSupply.toString()).toBe(mintInfo.supply.toString());
            });
        });
    });

    describe("Token2022 tests", () => {
        let tokenInMint: PublicKey;
        let tokenOutMint: PublicKey;

        beforeEach(async () => {
            // Create mints with different decimals to test precision handling
            tokenInMint = testHelper.createMint2022(6); // USDC-like (6 decimals)
            tokenOutMint = testHelper.createMint2022(9); // ONyc-like (9 decimals)

            // Initialize program and offers
            await program.initialize({ onycMint: tokenOutMint });

            // Create an offer
            await program.makeOffer({
                tokenInMint,
                tokenOutMint,
                tokenInProgram: TOKEN_2022_PROGRAM_ID
            });
        });

        it("Should calculate circulating supply correctly for Token2022", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector
            await program.addOfferVector({
                tokenInMint,
                tokenOutMint,
                baseTime: currentTime,
                basePrice: 2e9, // 2.0 with 9 decimals
                apr: 36_500, // 3.65% APR for fixed price
                priceFixDuration: 86400
            });

            // Should execute successfully with Token2022
            const circulatingSupply = await program.getCirculatingSupply({
                onycMint: tokenOutMint,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            const mintInfo = await testHelper.getMintInfo(tokenOutMint);
            expect(circulatingSupply.toString()).toBe(mintInfo.supply.toString());
        });
    });
});