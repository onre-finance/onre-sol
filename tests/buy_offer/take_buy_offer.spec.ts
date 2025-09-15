import { Keypair, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Take Buy Offer", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    let user: Keypair;

    let userTokenInAccount: PublicKey;
    let userTokenOutAccount: PublicKey;

    let bossTokenInAccount: PublicKey;

    let vaultTokenOutAccount: PublicKey;
    let vaultTokenInAccount: PublicKey;

    let offerId: number;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints with different decimals to test precision handling
        tokenInMint = testHelper.createMint(6); // USDC-like (6 decimals)
        tokenOutMint = testHelper.createMint(9); // ONyc-like (9 decimals)

        // Initialize program and offers
        await program.initialize();
        await program.initializeOffers();

        // Create a buy offer
        await program.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        const buyOfferAccount = await program.getBuyOfferAccount();
        const offer = buyOfferAccount.offers.find(o => o.offerId.toNumber() !== 0);
        offerId = offer.offerId.toNumber();

        // Initialize vault authority
        await program.initializeVaultAuthority();

        // Create token accounts
        user = testHelper.createUserAccount();
        userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(10_000e6), true);
        bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, testHelper.getBoss(), BigInt(0));
        userTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, user.publicKey);

        // Create and fund vault
        vaultTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, program.pdas.buyOfferVaultAuthorityPda, BigInt(0), true);
        vaultTokenInAccount = testHelper.createTokenAccount(tokenInMint, program.pdas.buyOfferVaultAuthorityPda, BigInt(0), true);

        // Fund vault
        testHelper.createTokenAccount(tokenOutMint, testHelper.getBoss(), BigInt(10_000e9));
        await program.buyOfferVaultDeposit({
            amount: 5_000e9,
            tokenMint: tokenOutMint
        });
    });

    describe("Price Calculation Tests", () => {
        it("Should calculate correct price in first interval", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector: base_price = 1.0 (1e9), APR = 3.65% (36500), duration = 1 day
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9, // 1.0 with 9 decimals
                apr: 36_500, // 3.65% APR (scaled by 1M)
                priceFixDuration: 86400 // 1 day
            });

            // Price in first interval should be: 1.0 * (1 + 0.0365 * (1 * 86400) / (365*24*3600))
            // = 1.0 * (1 + 0.0365 * 1/365) = 1.0 * 1.0001 = 1.0001

            const expectedTokenInAmount = 1_000_100; // 1.0001 USDC (6 decimals)

            await program.takeBuyOffer({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out (1e9)
            expect(userTokenOutBalanceAfter).toBe(BigInt(1e9));
        });

        it("Should calculate correct price with fee", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Create a new buy offer
            await program.makeBuyOffer({
                tokenInMint,
                tokenOutMint,
                feeBasisPoints: 100 // 1% fee
            });

            const feeOfferId = 2;
            await program.addBuyOfferVector({
                offerId: feeOfferId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Price in first interval should be: 1.0 * (1 + 0.0365 * (1 * 86400) / (365*24*3600))
            // = 1.0 * (1 + 0.0365 * 1/365) = 1.0 * 1.0001 = 1.0001
            const expectedTokenInAmount = 1_000_100; // 1.0001 USDC (6 decimals)

            await program.takeBuyOffer({
                offerId: feeOfferId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 0.9 token out
            expect(userTokenOutBalanceAfter).toBe(BigInt(99e7));
        });

        it("Should maintain price within same interval", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            const expectedTokenInAmount = 1_000_100;

            // First trade
            await program.takeBuyOffer({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            // Advance time within the same interval (less than 1 day)
            await testHelper.advanceClockBy(30_000); // 8 hours

            // Second trade - should use same price
            // Second user to workaround bankrun optimizing same transactions as one
            const user2 = testHelper.createUserAccount();
            const user2TokenInAccount = testHelper.createTokenAccount(tokenInMint, user2.publicKey, BigInt(10_000e6), true);
            const user2TokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user2.publicKey, BigInt(0), true);

            await program.takeBuyOffer({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user2.publicKey,
                signer: user2
            });

            const user1Balance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            const user2Balance = await testHelper.getTokenAccountBalance(user2TokenOutAccount);

            // Should receive another 1 token out
            expect(user1Balance).toBe(BigInt(1e9));
            expect(user2Balance).toBe(BigInt(1e9));
        });

        it("Should calculate higher price in second interval", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Advance to second interval
            await testHelper.advanceClockBy(86_400); // 1 day

            // Price in second interval: 1.0 * (1 + 0.0365 * (2 * 86400) / (365*24*3600))
            // = 1.0 * (1 + 0.0365 * 2/365) = 1.0 * 1.0002 = 1.0002

            const expectedTokenInAmount = 1_000_200; // 1.0002 USDC

            await program.takeBuyOffer({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out
            expect(userBalanceAfter).toBe(BigInt(1e9));
        });
    });

    describe("Multiple Vectors Tests", () => {
        it("Should use most recent active vector", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add first vector (past)
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 1000,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Add second vector (more recent)
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 2000,
                startPrice: 2e9, // Different start price
                apr: 73_000, // Different APR (7.3%)
                priceFixDuration: 86400
            });

            await testHelper.advanceClockBy(2500);

            // Should use the second vector's pricing
            // Price = 2.0 * (1 + 0.073 * 1/365) ≈ 2.0004
            const expectedTokenInAmount = 2_000_400;

            await program.takeBuyOffer({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out
            expect(userBalanceAfter).toBe(BigInt(1e9));
        });
    });

    describe("Error Cases", () => {
        it("Should fail when offer does not exist", async () => {
            const nonExistentOfferId = 999;

            await expect(
                program.takeBuyOffer({
                    offerId: nonExistentOfferId,
                    tokenInAmount: 1_000_000,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                })
            ).rejects.toThrow("Offer not found");
        });

        it("Should fail when no active vector exists", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector in the future
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime + 10000, // Future start time
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            await expect(
                program.takeBuyOffer({
                    offerId,
                    tokenInAmount: 1_000_000,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                })
            ).rejects.toThrow("No active vector");
        });

        it("Should fail with insufficient user token balance", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Try to spend more than user has (user has 10,000 USDC)
            const excessiveAmount = 20_000e6;

            await expect(
                program.takeBuyOffer({
                    offerId,
                    tokenInAmount: excessiveAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                })
            ).rejects.toThrow("insufficient funds");
        });

        it("Should fail with insufficient vault balance", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector with very low price (expensive for vault)
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e6, // Very low price = 0.001 USDC per token
                apr: 0, // Zero APR for fixed price
                priceFixDuration: 86400
            });

            // This would require giving out 10,000 tokens for 10 USDC, but vault only has 5,000
            const tokenInAmount = 10e6; // 10 USDC

            await expect(
                program.takeBuyOffer({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                })
            ).rejects.toThrow("insufficient funds");
        });
    });

    describe("Token Transfer Tests", () => {
        it("Should correctly transfer tokens between accounts", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            const tokenInAmount = 1_000_100;

            const userTokenInBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
            const bossTokenInBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
            const vaultTokenOutBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

            await program.takeBuyOffer({
                offerId,
                tokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userTokenInAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);
            const userTokenOutAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            const bossTokenInAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);
            const vaultTokenOutAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

            // Verify transfers
            expect(userTokenInBefore - userTokenInAfter).toBe(BigInt(tokenInAmount));
            expect(userTokenOutAfter).toBe(BigInt(1e9));
            expect(bossTokenInAfter - bossTokenInBefore).toBe(BigInt(tokenInAmount));
            expect(vaultTokenOutBefore - vaultTokenOutAfter).toBe(BigInt(1e9));
        });
    });

    describe("Edge Cases", () => {
        it("Should handle zero APR (fixed price) correctly", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 0, // Zero APR for fixed price
                priceFixDuration: 86400
            });

            // Price should remain almost constant with minimal APR
            await testHelper.advanceClockBy(86_401 * 10); // 10 days

            // With 0.0001% APR over 10 days: price ≈ 1.000000027 ≈ 1.0 USDC
            const expectedTokenInAmount = 1_000_000; // Exactly 1.0 USDC

            await program.takeBuyOffer({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out
            expect(userBalanceAfter).toEqual(BigInt(1_000_000_000));
        });

        it("Should handle high APR over long time period with precision", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 365_000, // 36.5% yearly APR
                priceFixDuration: 86400
            });

            // Advance 1 year (365 days)
            await testHelper.advanceClockBy(86400 * 365);

            // After 1 year with 36.5% APR: price = 1.0 * (1 + 0.365) = 1.365
            // But due to discrete intervals, it uses (366 * D) / S formula
            // Let's calculate the actual expected price and use a tolerance
            const expectedTokenInAmount = 1_366_000; // Based on the actual calculation from logs

            await program.takeBuyOffer({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out
            expect(userBalanceAfter).toEqual(BigInt(1_000_000_000));
        });
    });

    describe("Mint/Transfer Integration Tests", () => {
        describe("Program lacks mint authority tests", () => {
            it("Should transfer token_out tokens from vault to user when program lacks mint authority", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                await program.addBuyOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 36_500,
                    priceFixDuration: 86400
                });

                const tokenInAmount = 1.0001e6;
                const vaultBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                // Execute take_buy_offer without mint authority (should use vault transfer)
                await program.takeBuyOffer({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                // Verify tokens were transferred from vault to user
                const userReceived = await testHelper.getTokenAccountBalance(userTokenOutAccount);
                const vaultBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                const vaultDeducted = vaultBalanceBefore - vaultBalanceAfter;

                expect(userReceived).toBe(BigInt(1e9));
                expect(vaultDeducted).toBe(BigInt(1e9));
            });

            it("Should transfer token_in tokens from user to boss when program lacks mint authority", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                await program.addBuyOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 36_500,
                    priceFixDuration: 86400
                });

                const tokenInAmount = 1.0001e6;
                const bossTokenInBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                const userTokenInBalanceBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);

                // Execute take_buy_offer without mint authority (should use vault transfer)
                await program.takeBuyOffer({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                // Verify tokens were transferred from vault to user
                const bossTokenInBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                const userTokenInBalanceAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);

                const bossReceived = bossTokenInBalanceAfter - bossTokenInBalanceBefore;
                const userPaid = userTokenInBalanceBefore - userTokenInBalanceAfter;

                expect(bossReceived).toBe(BigInt(1.0001e6));
                expect(userPaid).toBe(BigInt(1.0001e6));
            });
        });

        describe("Program has mint authority tests", () => {
            it("Should mint token_out tokens directly to user when program has mint authority", async () => {
                // Transfer mint authority from boss to program for tokenOutMint
                await program.transferMintAuthorityToProgram({
                    mint: tokenOutMint
                });

                const currentTime = await testHelper.getCurrentClockTime();

                await program.addBuyOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 36_500,
                    priceFixDuration: 86400
                });

                const tokenInAmount = 1.0001e6;

                const vaultBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);
                const bossTokenInBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);

                // Execute take_buy_offer with mint authority (should mint directly)
                await program.takeBuyOffer({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                // Verify tokens were minted to user (vault balance unchanged)
                const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
                const vaultBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);
                const bossTokenInBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);

                const vaultChange = vaultBalanceAfter - vaultBalanceBefore;
                const bossTokenInChange = bossTokenInBalanceAfter - bossTokenInBalanceBefore;

                expect(userTokenOutBalance).toBe(BigInt(1e9)); // Should receive 1 token out
                expect(bossTokenInChange).toBe(BigInt(tokenInAmount)); // token_in tokens are transferred to boss
                expect(vaultChange).toEqual(BigInt(0)); // Vault unchanged
            });

            it("Should burn token_out tokens when program has mint authority", async () => {
                // Transfer mint authority from boss to program for tokenOutMint
                await program.transferMintAuthorityToProgram({
                    mint: tokenInMint
                });

                const currentTime = await testHelper.getCurrentClockTime();

                await program.addBuyOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 0,
                    priceFixDuration: 86400
                });

                const tokenInAmount = 1.0001e6;

                const bossTokenInBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                const vaultBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenInAccount);

                // Execute take_buy_offer with mint authority (should mint directly)
                await program.takeBuyOffer({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                // Verify token_in tokens were burned (boss account balance unchanged)
                const bossTokenInBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                const vaultBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenInAccount);
                const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount);

                const vaultChange = vaultBalanceAfter - vaultBalanceBefore;
                const bossChange = bossTokenInBalanceAfter - bossTokenInBalanceBefore;

                expect(userTokenOutBalance).toBeGreaterThan(BigInt(1e9)); // Should receive 1 token out
                expect(bossChange).toEqual(BigInt(0)); // Boss unchanged
                expect(vaultChange).toEqual(BigInt(0)); // Vault unchanged

            });
        });

        describe("Edge Cases", () => {
            it("Should handle fee calculations correctly when minting", async () => {
                // Create an offer with fees
                await program.makeBuyOffer({
                    tokenInMint,
                    tokenOutMint,
                    feeBasisPoints: 500 // 5% fee
                });

                // Transfer mint authority to program
                await program.transferMintAuthorityToProgram({
                    mint: tokenOutMint
                });

                const currentTime = await testHelper.getCurrentClockTime();

                await program.addBuyOfferVector({
                    offerId: 2,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 0,
                    priceFixDuration: 86400
                });

                const tokenInAmount = 1_050_000; // 1.05 USDC (includes 5% fee)
                const feeUser = testHelper.createUserAccount();
                const feeUserTokenInAccount = testHelper.createTokenAccount(tokenInMint, feeUser.publicKey, BigInt(10_000e6), true);

                const bossBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);

                await program.takeBuyOffer({
                    offerId: 2,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: feeUser.publicKey,
                    signer: feeUser
                });

                // Verify boss received full payment including fee
                const bossAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                expect(bossAfter - bossBefore).toEqual(BigInt(1_050_000)); // Full amount with fee

                // Verify user received correct token_out amount (based on net amount after fee)
                const feeUserTokenOutAccount = getAssociatedTokenAddressSync(
                    tokenOutMint,
                    feeUser.publicKey,
                    true
                );

                const userBalance = await testHelper.getTokenAccountBalance(feeUserTokenOutAccount);
                expect(userBalance).toEqual(BigInt(997_500_000)); // 0.9975 token out (based on 0.9975 USDC after 5% fee)
            });
        });
    });
});