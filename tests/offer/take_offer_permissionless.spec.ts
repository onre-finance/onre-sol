import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

describe("Take Offer Permissionless", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    let user: Keypair;

    let userTokenInAccount: PublicKey;
    let userTokenOutAccount: PublicKey;

    let bossTokenInAccount: PublicKey;
    let bossTokenOutAccount: PublicKey;

    let vaultTokenOutAccount: PublicKey;
    let vaultTokenInAccount: PublicKey;

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

        // Create an offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        offerId = 1;

        // Initialize
        await program.initializeVaultAuthority();
        await program.initializePermissionlessAccount({
            accountName: "test-account"
        });
        await program.initializeKillSwitchState();

        // Create token accounts
        user = testHelper.createUserAccount();
        userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(10_000e6), true);
        bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, testHelper.getBoss(), BigInt(0));
        userTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, user.publicKey);
        bossTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, testHelper.getBoss());

        // Create vault and permissionless intermediary accounts
        vaultTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, program.pdas.offerVaultAuthorityPda, BigInt(0), true);
        vaultTokenInAccount = testHelper.createTokenAccount(tokenInMint, program.pdas.offerVaultAuthorityPda, BigInt(0), true);

        permissionlessTokenInAccount = testHelper.createTokenAccount(tokenInMint, program.pdas.permissionlessVaultAuthorityPda, BigInt(0), true);
        permissionlessTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, program.pdas.permissionlessVaultAuthorityPda, BigInt(0), true);

        // Fund vault
        testHelper.createTokenAccount(tokenOutMint, testHelper.getBoss(), BigInt(10_000e9));
        await program.offerVaultDeposit({
            amount: 5_000e9,
            tokenMint: tokenOutMint
        });
    });

    describe("Basic Functionality Tests", () => {
        it("Should successfully execute permissionless offer", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector: base_price = 1.0 (1e9), APR = 3.65% (36500), duration = 1 day
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            const expectedTokenInAmount = 1.0001e6; // 1.0001 USDC

            // Execute permissionless offer
            await program.takeOfferPermissionless({
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

            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            const tokenInAmount = 1_000_100;

            // Get balances before transaction
            const userTokenInBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
            const bossTokenInBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
            const vaultTokenOutBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);
            const permissionlessTokenInBefore = await testHelper.getTokenAccountBalance(permissionlessTokenInAccount);
            const permissionlessTokenOutBefore = await testHelper.getTokenAccountBalance(permissionlessTokenOutAccount);

            await program.takeOfferPermissionless({
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
            expect(userTokenInBefore - userTokenInAfter).toBe(BigInt(tokenInAmount)); // User paid token_in
            expect(userTokenOutAfter).toBe(BigInt(1e9)); // User received token_out
            expect(bossTokenInAfter - bossTokenInBefore).toBe(BigInt(tokenInAmount)); // Boss received token_in
            expect(vaultTokenOutBefore - vaultTokenOutAfter).toBe(BigInt(1e9)); // Vault gave token_out

            // Verify intermediary accounts are empty (no residual balances)
            expect(permissionlessTokenInAfter).toBe(permissionlessTokenInBefore); // Should be 0
            expect(permissionlessTokenOutAfter).toBe(permissionlessTokenOutBefore); // Should be 0
        });

        it("Should APR identical results to direct take_offer", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addOfferVector({
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

            // Execute permissionless flow
            await program.takeOfferPermissionless({
                offerId,
                tokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            // Execute direct flow
            await program.takeOffer({
                offerId,
                tokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user2.publicKey,
                signer: user2
            });

            const user2TokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, user.publicKey);

            const user1BalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            const user2BalanceAfter = await testHelper.getTokenAccountBalance(user2TokenOutAccount);

            // Both should receive identical amounts
            expect(user1BalanceAfter).toBe(user2BalanceAfter);
        });

        it("Should leave intermediary accounts with zero balance after large transaction", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add more funding to vault for large transaction
            const bossTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, testHelper.getBoss(), BigInt(50_000e9));
            await program.offerVaultDeposit({
                amount: 25_000e9,
                tokenMint: tokenOutMint
            });

            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Large transaction
            const largeTokenInAmount = 5_000_500_000; // 5000.5 USDC

            await program.takeOfferPermissionless({
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

    describe("Price Calculation Tests", () => {
        it("Should calculate correct price in first interval", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            const expectedTokenInAmount = 1.0001e6; // 1.0001 USDC

            await program.takeOfferPermissionless({
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

            // Create a new offer
            await program.makeOffer({
                tokenInMint,
                tokenOutMint,
                feeBasisPoints: 100 // 1% fee
            });

            const feeOfferId = 2;
            await program.addOfferVector({
                offerId: feeOfferId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Price in first interval should be: 1.0 * (1 + 0.0365 * (1 * 86400) / (365*24*3600))
            // = 1.0 * (1 + 0.0365 * 1/365) = 1.0 * 1.0001 = 1.0001
            const expectedTokenInAmount = 1_000_100; // 1.0001 USDC (6 decimals)

            await program.takeOfferPermissionless({
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

            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            const expectedTokenInAmount = 1_000_100;

            // First trade
            await program.takeOfferPermissionless({
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

            await program.takeOfferPermissionless({
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

            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Advance to second interval
            await testHelper.advanceClockBy(86_400); // 1 day

            const expectedTokenInAmount = 1.0002e6; // 1.0002 USDC

            await program.takeOfferPermissionless({
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

        it("Should use most recent active vector", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add first vector (past)
            await program.addOfferVector({
                offerId,
                startTime: currentTime + 1000,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Add second vector (more recent)
            await program.addOfferVector({
                offerId,
                startTime: currentTime + 2000,
                startPrice: 2e9, // Different start price
                apr: 73_000, // Different APR (7.3%)
                priceFixDuration: 86400
            });

            await testHelper.advanceClockBy(2500);

            // Should use the second vector's pricing
            const expectedTokenInAmount = 2.0004e6;

            await program.takeOfferPermissionless({
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

    describe("Error Handling Tests", () => {
        it("Should fail when offer does not exist", async () => {
            const nonExistentOfferId = 999;

            await expect(
                program.takeOfferPermissionless({
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
            await program.addOfferVector({
                offerId,
                startTime: currentTime + 10000, // Future start time
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            await expect(
                program.takeOfferPermissionless({
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

            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Try to spend more than user has (user has 10,000 USDC)
            const excessiveAmount = 20_000e6;

            await expect(
                program.takeOfferPermissionless({
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
            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e6, // Very low price = 0.001 USDC per token
                apr: 0, // Zero APR for fixed price
                priceFixDuration: 86400
            });

            // This would require giving out 10,000 tokens for 10 USDC, but vault only has 5,000
            const tokenInAmount = 10e6; // 10 USDC

            await expect(
                program.takeOfferPermissionless({
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

    describe("Edge Cases", () => {
        it("Should handle zero APR (fixed price) correctly", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 0, // Zero APR for fixed price
                priceFixDuration: 86400
            });

            // Advance time significantly
            await testHelper.advanceClockBy(86_401 * 10); // 10 days

            const expectedTokenInAmount = 1_000_000; // Exactly 1.0 USDC

            await program.takeOfferPermissionless({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            expect(userBalanceAfter).toEqual(BigInt(1_000_000_000));
        });

        it("Should handle high APR over long time period with precision", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 365_000, // 36.5% yearly APR
                priceFixDuration: 86400
            });

            // Advance 1 year (365 days)
            await testHelper.advanceClockBy(86400 * 365);

            const expectedTokenInAmount = 1_366_000;

            await program.takeOfferPermissionless({
                offerId,
                tokenInAmount: expectedTokenInAmount,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            expect(userBalanceAfter).toEqual(BigInt(1_000_000_000));
        });
    });

    describe("Mint/Transfer Integration Tests", () => {
        describe("Program lacks mint authority tests", () => {
            it("Should transfer token_out tokens from vault to user when program lacks mint authority", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 36_500,
                    priceFixDuration: 86400
                });

                const tokenInAmount = 1.0001e6;

                const mintInfoBefore = await testHelper.getMintInfo(tokenInMint);
                // Token out before
                const bossTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenOutAccount);
                const vaultTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                // Execute take_offer without mint authority (should use vault transfer)
                await program.takeOfferPermissionless({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                const mintInfoAfter = await testHelper.getMintInfo(tokenInMint);
                // Token out after
                const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
                const bossTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenOutAccount);
                const vaultTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);
                const permissionlessTokenOutBalance = await testHelper.getTokenAccountBalance(permissionlessTokenOutAccount);

                // Verify token changes
                const supplyBurned = mintInfoBefore.supply - mintInfoAfter.supply;
                const userReceived = userTokenOutBalance;
                const vaultDeducted = vaultTokenOutBalanceBefore - vaultTokenOutBalanceAfter;
                const bossPaid = bossTokenOutBalanceAfter - bossTokenOutBalanceBefore;

                expect(supplyBurned).toBe(BigInt(0)); // No supply burned
                expect(userReceived).toBe(BigInt(1e9)); // Should receive 1 token out
                expect(vaultDeducted).toBe(BigInt(1e9)); // Vault gave token_out
                expect(bossPaid).toEqual(BigInt(0)); // Boss no change (transferred from vault)
                expect(permissionlessTokenOutBalance).toBe(BigInt(0)); // No residual balance
            });

            it("Should transfer token_in tokens from user to boss when program lacks mint authority", async () => {
                const currentTime = await testHelper.getCurrentClockTime();

                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 36_500,
                    priceFixDuration: 86400
                });

                const tokenInAmount = 1.0001e6;

                const mintInfoBefore = await testHelper.getMintInfo(tokenInMint);
                // Token in before
                const userTokenInBalanceBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
                const bossTokenInBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                const vaultTokenInBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenInAccount);

                // Execute take_offer without mint authority (should use vault transfer)
                await program.takeOfferPermissionless({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                const mintInfoAfter = await testHelper.getMintInfo(tokenInMint);
                // Token in after
                const userTokenInBalanceAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);
                const bossTokenInBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                const vaultTokenInBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenInAccount);
                const permissionlessTokenInBalance = await testHelper.getTokenAccountBalance(permissionlessTokenInAccount);

                // Verify token changes
                const supplyBurned = mintInfoBefore.supply - mintInfoAfter.supply;
                const userPaid = userTokenInBalanceBefore - userTokenInBalanceAfter;
                const vaultReceived = vaultTokenInBalanceAfter - vaultTokenInBalanceBefore;
                const bossReceived = bossTokenInBalanceAfter - bossTokenInBalanceBefore;

                expect(supplyBurned).toBe(BigInt(0)); // No supply burned
                expect(userPaid).toBe(BigInt(tokenInAmount)); // User paid token_in amount
                expect(vaultReceived).toBe(BigInt(0)); // Vault received no tokens (transferred to boss)
                expect(bossReceived).toEqual(BigInt(tokenInAmount)); // Boss received token_in tokens
                expect(permissionlessTokenInBalance).toBe(BigInt(0)); // No residual balance
            });
        });

        describe("Program has mint authority tests", () => {
            it("Should mint token_out tokens directly to user when program has mint authority", async () => {
                // Transfer mint authority from boss to program for tokenOutMint
                await program.transferMintAuthorityToProgram({
                    mint: tokenOutMint
                });

                const currentTime = await testHelper.getCurrentClockTime();

                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 36_500,
                    priceFixDuration: 86400
                });

                const tokenInAmount = 1.0001e6;

                const mintInfoBefore = await testHelper.getMintInfo(tokenInMint);
                // Token out before
                const bossTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenOutAccount);
                const vaultTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                // Execute take_offer with mint authority (should mint directly)
                await program.takeOfferPermissionless({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                const mintInfoAfter = await testHelper.getMintInfo(tokenInMint);
                // Token out after
                const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
                const bossTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenOutAccount);
                const vaultTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);
                const permissionlessTokenOutBalance = await testHelper.getTokenAccountBalance(permissionlessTokenOutAccount);

                // Verify token changes
                const supplyBurned = mintInfoBefore.supply - mintInfoAfter.supply;
                const userReceived = userTokenOutBalance;
                const vaultDeducted = vaultTokenOutBalanceBefore - vaultTokenOutBalanceAfter;
                const bossPaid = bossTokenOutBalanceAfter - bossTokenOutBalanceBefore;

                expect(supplyBurned).toBe(BigInt(0)); // No supply burned
                expect(userReceived).toBe(BigInt(1e9)); // Should receive 1 token out
                expect(vaultDeducted).toBe(BigInt(0)); // No change to Vault (tokens were minted)
                expect(bossPaid).toEqual(BigInt(0)); // Boss no change (tokens were minted)
                expect(permissionlessTokenOutBalance).toBe(BigInt(0)); // No residual balance
            });

            it("Should burn token_in tokens when program has mint authority", async () => {
                // Transfer mint authority from boss to program for tokenOutMint
                await program.transferMintAuthorityToProgram({
                    mint: tokenInMint
                });

                const currentTime = await testHelper.getCurrentClockTime();

                await program.addOfferVector({
                    offerId,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 0,
                    priceFixDuration: 86400
                });

                const tokenInAmount = 1.0001e6;

                const mintInfoBefore = await testHelper.getMintInfo(tokenInMint);
                // Token in before
                const userTokenInBalanceBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
                const bossTokenInBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                const vaultTokenInBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenInAccount);

                // Execute take_offer with mint authority (should mint directly)
                await program.takeOfferPermissionless({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                // Verify token_in tokens were burned (boss account balance unchanged)
                const mintInfoAfter = await testHelper.getMintInfo(tokenInMint);
                // Token in after
                const userTokenInBalanceAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);
                const bossTokenInBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                const vaultTokenInBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenInAccount);
                const permissionlessTokenInBalance = await testHelper.getTokenAccountBalance(permissionlessTokenInAccount);

                const supplyBurned = mintInfoBefore.supply - mintInfoAfter.supply;
                const userPaid = userTokenInBalanceBefore - userTokenInBalanceAfter;
                const vaultReceived = vaultTokenInBalanceAfter - vaultTokenInBalanceBefore;
                const bossReceived = bossTokenInBalanceAfter - bossTokenInBalanceBefore;

                expect(supplyBurned).toBe(BigInt(tokenInAmount));
                expect(userPaid).toBe(BigInt(tokenInAmount)); // User paid token_in amount
                expect(vaultReceived).toBe(BigInt(0)); // Vault received no tokens (burned)
                expect(bossReceived).toEqual(BigInt(0)); // Boss received no tokens (burned)
                expect(permissionlessTokenInBalance).toBe(BigInt(0)); // No residual balance
            });
        });

        describe("Edge Cases", () => {
            it("Should handle fee calculations correctly when minting", async () => {
                // Create an offer with fees
                await program.makeOffer({
                    tokenInMint,
                    tokenOutMint,
                    feeBasisPoints: 500 // 5% fee
                });

                // Transfer mint authority to program
                await program.transferMintAuthorityToProgram({
                    mint: tokenOutMint
                });

                const currentTime = await testHelper.getCurrentClockTime();

                await program.addOfferVector({
                    offerId: 2,
                    startTime: currentTime,
                    startPrice: 1e9,
                    apr: 0,
                    priceFixDuration: 86400
                });

                const tokenInAmount = 1_050_000; // 1.05 USDC (includes 5% fee)
                const feeUser = testHelper.createUserAccount();
                const feeUserTokenInAccount = testHelper.createTokenAccount(tokenInMint, feeUser.publicKey, BigInt(10_000e6), true);

                await program.takeOfferPermissionless({
                    offerId: 2,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: feeUser.publicKey,
                    signer: feeUser
                });

                // Verify boss received full payment including fee
                const bossBalance = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                expect(bossBalance).toEqual(BigInt(1_050_000)); // Full amount with fee

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

    describe("Kill Switch Tests", () => {
        it("Should reject take_offer_permissionless when kill switch is enabled", async () => {
            // Initialize admin state and add an admin
            await program.initializeAdminState();
            const admin = testHelper.createUserAccount();
            await program.addAdmin({ admin: admin.publicKey });

            const currentTime = await testHelper.getCurrentClockTime();

            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Enable kill switch
            await program.killSwitch({ enable: true, signer: admin });

            // Verify kill switch is enabled
            const killSwitchState = await program.getKillSwitchState();
            expect(killSwitchState.isKilled).toBe(true);

            // Try to take offer permissionless - should fail
            await expect(
                program.takeOfferPermissionless({
                    offerId,
                    tokenInAmount: 1_000_100,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                })
            ).rejects.toThrow("Kill switch is activated");
        });

        it("Should allow take_offer_permissionless when kill switch is disabled", async () => {
            // Initialize admin state and add an admin
            await program.initializeAdminState();
            const admin = testHelper.createUserAccount();
            await program.addAdmin({ admin: admin.publicKey });

            const currentTime = await testHelper.getCurrentClockTime();

            await program.addOfferVector({
                offerId,
                startTime: currentTime,
                startPrice: 1e9,
                apr: 36_500,
                priceFixDuration: 86400
            });

            // Enable then disable kill switch
            await program.killSwitch({ enable: true, signer: admin });
            await program.killSwitch({ enable: false }); // Only boss can disable

            // Verify kill switch is disabled
            const killSwitchState = await program.getKillSwitchState();
            expect(killSwitchState.isKilled).toBe(false);

            // Take offer permissionless - should succeed
            await program.takeOfferPermissionless({
                offerId,
                tokenInAmount: 1_000_100,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            });

            // Verify user received tokens
            const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            expect(userTokenOutBalance).toBe(BigInt(1e9));
        });
    });

});