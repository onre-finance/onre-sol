import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Take Buy Offer Permissionless", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;
    let user: any;
    let userTokenInAccount: PublicKey;
    let userTokenOutAccount: PublicKey;
    let bossTokenInAccount: PublicKey;
    let vaultTokenOutAccount: PublicKey;
    let permissionlessAuthorityPda: PublicKey;
    let permissionlessTokenInAccount: PublicKey;
    let permissionlessTokenOutAccount: PublicKey;
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

        offerId = 1;

        // Initialize
        await program.initializeVaultAuthority();
        await program.initializePermissionlessAccount({
            accountName: "test-account"
        });

        // Create user accounts
        user = testHelper.createUserAccount();
        userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(10_000e6), true);
        userTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(0), true);
        bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, testHelper.getBoss(), BigInt(0));

        // Create vault and permissionless intermediary accounts
        vaultTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, program.pdas.buyOfferVaultAuthorityPda, BigInt(0), true);
        permissionlessTokenInAccount = testHelper.createTokenAccount(tokenInMint, program.pdas.permissionlessVaultAuthorityPda, BigInt(0), true);
        permissionlessTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, program.pdas.permissionlessVaultAuthorityPda, BigInt(0), true);

        // Fund vault
        testHelper.createTokenAccount(tokenOutMint, testHelper.getBoss(), BigInt(10_000e9));
        await program.buyOfferVaultDeposit({
            amount: 5_000e9,
            tokenMint: tokenOutMint
        });
    });

    describe("Basic Functionality Tests", () => {
        it("Should successfully execute permissionless buy offer", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector: base_price = 1.0 (1e9), APR = 3.65% (36500), duration = 1 day
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            const expectedTokenInAmount = 1_000_100; // 1.0001 USDC

            // Execute permissionless buy offer
            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            // Verify user received tokens
            const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            expect(userTokenOutBalance).toBe(BigInt(1e9));
        });

        it("Should complete 4-step transfer process correctly", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            const tokenInAmount = 1_000_100;

            // Get balances before transaction
            const userTokenInBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
            const userTokenOutBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            const bossTokenInBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
            const vaultTokenOutBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);
            const permissionlessTokenInBefore = await testHelper.getTokenAccountBalance(permissionlessTokenInAccount);
            const permissionlessTokenOutBefore = await testHelper.getTokenAccountBalance(permissionlessTokenOutAccount);

            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            // Get balances after transaction
            const userTokenInAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);
            const userTokenOutAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            const bossTokenInAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);
            const vaultTokenOutAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);
            const permissionlessTokenInAfter = await testHelper.getTokenAccountBalance(permissionlessTokenInAccount);
            const permissionlessTokenOutAfter = await testHelper.getTokenAccountBalance(permissionlessTokenOutAccount);

            // Verify all transfers
            // expect(userTokenInBefore - userTokenInAfter).toBe(BigInt(tokenInAmount)); // User paid token_in
            expect(userTokenOutAfter - userTokenOutBefore).toBe(BigInt(1e9)); // User received token_out
            expect(bossTokenInAfter - bossTokenInBefore).toBe(BigInt(tokenInAmount)); // Boss received token_in
            expect(vaultTokenOutBefore - vaultTokenOutAfter).toBe(BigInt(1e9)); // Vault gave token_out

            // Verify intermediary accounts are empty (no residual balances)
            expect(permissionlessTokenInAfter).toBe(permissionlessTokenInBefore); // Should be 0
            expect(permissionlessTokenOutAfter).toBe(permissionlessTokenOutBefore); // Should be 0
        });

        it("Should APR identical results to direct take_buy_offer", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            const tokenInAmount = 1_000_100;

            // Create second user for direct comparison
            const user2 = testHelper.createUserAccount();
            const user2TokenInAccount = testHelper.createTokenAccount(tokenInMint, user2.publicKey, BigInt(10_000e6), true);
            const user2TokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user2.publicKey, BigInt(0), true);

            // Execute both transactions at the same time
            const user1BalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            const user2BalanceBefore = await testHelper.getTokenAccountBalance(user2TokenOutAccount);

            // Execute permissionless flow
            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            // Execute direct flow
            await program.takeBuyOffer({
                offerId,
                tokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user2.publicKey,
                signer: user2
            });

            const user1BalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            const user2BalanceAfter = await testHelper.getTokenAccountBalance(user2TokenOutAccount);

            // Both should receive identical amounts
            const user1Received = user1BalanceAfter - user1BalanceBefore;
            const user2Received = user2BalanceAfter - user2BalanceBefore;
            expect(user1Received).toBe(user2Received);
        });
    });

    describe("Price Calculation Tests", () => {
        it("Should calculate correct price in first interval", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            const expectedTokenInAmount = 1_000_100; // 1.0001 USDC

            const userTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out (1e9)
            expect(userTokenOutBalanceAfter - userTokenOutBalanceBefore).toBe(BigInt(1e9));
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

            await program.takeBuyOfferPermissionless({
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
            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            // Advance time within the same interval (less than 1 day)
            await testHelper.advanceClockBy(30_000); // 8 hours

            // Second trade with different user
            const user2 = testHelper.createUserAccount();
            const user2TokenInAccount = testHelper.createTokenAccount(tokenInMint, user2.publicKey, BigInt(10_000e6), true);
            const user2TokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user2.publicKey, BigInt(0), true);

            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user2.publicKey,
                signer: user2
            });

            const user1Balance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            const user2Balance = await testHelper.getTokenAccountBalance(user2TokenOutAccount);

            // Both should receive 1 token out
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

            const expectedTokenInAmount = 1_000_200; // 1.0002 USDC

            const userBalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out
            expect(userBalanceAfter - userBalanceBefore).toBe(BigInt(1e9));
        });

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
            const expectedTokenInAmount = 2_000_400;

            const userBalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out
            expect(userBalanceAfter - userBalanceBefore).toBe(BigInt(1e9));
        });
    });

    describe("Error Handling Tests", () => {
        it("Should fail when offer does not exist", async () => {
            const nonExistentOfferId = 999;

            await expect(
                program.takeBuyOfferPermissionless({
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
                program.takeBuyOfferPermissionless({
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
                program.takeBuyOfferPermissionless({
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
                program.takeBuyOfferPermissionless({
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

    describe("Permissionless-Specific Tests", () => {
        it("Should maintain precision through 4-step transfer with different decimals", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Use a more complex price to test precision
            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1_234_567_890, // 1.23456789 with 9 decimals
                apr: 45_670, // 4.567% yearly APR
                priceFixDuration: 86400
            });

            const tokenInAmount = 1_234_688; // Calculated amount in USDC (6 decimals)

            const userTokenOutBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userTokenOutAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive close to 1 token out with proper precision handling
            const receivedTokens = userTokenOutAfter - userTokenOutBefore;
            expect(receivedTokens).toBeGreaterThan(BigInt(990_000_000)); // Allow for small rounding
            expect(receivedTokens).toBeLessThan(BigInt(1_010_000_000));
        });

        it("Should leave intermediary accounts with zero balance after large transaction", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add more funding to vault for large transaction
            const bossTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, testHelper.getBoss(), BigInt(50_000e9));
            await program.buyOfferVaultDeposit({
                amount: 25_000e9,
                tokenMint: tokenOutMint
            });

            await program.addBuyOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Large transaction
            const largeTokenInAmount = 5_000_500_000; // 5000.5 USDC

            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount: largeTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            // Verify intermediary accounts are empty
            const permissionlessTokenInBalance = await testHelper.getTokenAccountBalance(permissionlessTokenInAccount);
            const permissionlessTokenOutBalance = await testHelper.getTokenAccountBalance(permissionlessTokenOutAccount);

            expect(permissionlessTokenInBalance).toBe(BigInt(0));
            expect(permissionlessTokenOutBalance).toBe(BigInt(0));

            // Verify user received the expected large amount
            const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            expect(userTokenOutBalance).toBeGreaterThanOrEqual(BigInt(5000e9)); // Should receive ~5000 tokens
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

            // Advance time significantly
            await testHelper.advanceClockBy(86_401 * 10); // 10 days

            const expectedTokenInAmount = 1_000_000; // Exactly 1.0 USDC

            const userBalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            const receivedTokens = userBalanceAfter - userBalanceBefore;
            expect(receivedTokens).toEqual(BigInt(1_000_000_000));
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

            const expectedTokenInAmount = 1_366_000;

            const userBalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            await program.takeBuyOfferPermissionless({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            const receivedTokens = userBalanceAfter - userBalanceBefore;
            expect(receivedTokens).toEqual(BigInt(1_000_000_000));
        });
    });
});