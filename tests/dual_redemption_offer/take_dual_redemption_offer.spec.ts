import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

describe("Take dual redemption offer", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    const offerId = 1;

    let tokenInMint: PublicKey;
    let tokenOutMint1: PublicKey;
    let tokenOutMint2: PublicKey;

    let user: Keypair;
    let userTokenInAccount: PublicKey;
    let userTokenOutAccount1: PublicKey;
    let userTokenOutAccount2: PublicKey;

    let boss: PublicKey;
    let bossTokenInAccount: PublicKey;
    let bossTokenOutAccount1: PublicKey;
    let bossTokenOutAccount2: PublicKey;

    let vaultTokenInAccount: PublicKey;
    let vaultTokenOutAccount1: PublicKey;
    let vaultTokenOutAccount2: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        user = testHelper.createUserAccount();
        boss = testHelper.getBoss();

        // Initialize program, offers, and vault authority
        await program.initialize();
        await program.initializeOffers();
        await program.initializeVaultAuthority();

        // Create token mints
        tokenInMint = testHelper.createMint(9);
        tokenOutMint1 = testHelper.createMint(9);
        tokenOutMint2 = testHelper.createMint(6);

        // Set up vault with tokens
        bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        bossTokenOutAccount1 = testHelper.createTokenAccount(tokenOutMint1, boss, BigInt(1000e9)); // 1000 tokens for boss
        bossTokenOutAccount2 = testHelper.createTokenAccount(tokenOutMint2, boss, BigInt(1000e6)); // 1000 tokens for boss

        userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(1000e9)); // 1000 tokens for user
        userTokenOutAccount1 = getAssociatedTokenAddressSync(tokenOutMint1, user.publicKey);
        userTokenOutAccount2 = getAssociatedTokenAddressSync(tokenOutMint2, user.publicKey);

        vaultTokenInAccount = testHelper.createTokenAccount(tokenInMint, program.pdas.dualRedemptionVaultAuthorityPda, BigInt(0), true);
        vaultTokenOutAccount1 = getAssociatedTokenAddressSync(tokenOutMint1, program.pdas.dualRedemptionVaultAuthorityPda, true);
        vaultTokenOutAccount2 = getAssociatedTokenAddressSync(tokenOutMint2, program.pdas.dualRedemptionVaultAuthorityPda, true);

        // Deposit tokens to vault
        await program.dualRedemptionVaultDeposit({
            amount: 1000e9, // 1000 tokens with 9 decimals
            tokenMint: tokenOutMint1
        });

        await program.dualRedemptionVaultDeposit({
            amount: 1000e6, // 1000 tokens with 6 decimals
            tokenMint: tokenOutMint2
        });
    });

    test("Take dual redemption offer with 80/20 ratio should succeed", async () => {
        const startTime = await testHelper.getCurrentClockTime();

        await program.makeDualRedemptionOffer({
            startTime,
            endTime: startTime + 3600,
            price1: 2e9,
            price2: 1e9,
            ratioBasisPoints: 8000,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // when - user takes offer: pays 100 token_in
        // Expected calculation:
        // - 80% of 100 = 80 tokens go to token_out_1 at price 2.0 → 80/2 = 40 token_out_1
        // - 20% of 100 = 20 tokens go to token_out_2 at price 1.0 → 20/1 = 20 token_out_2
        const tokenInAmount = 100e9; // 100 tokens with 9 decimals

        await program.takeDualRedemptionOffer({
            offerId,
            tokenInAmount,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2,
            user: user.publicKey,
            signer: user
        });

        // then - verify balances
        // User should have: 1000 - 100 = 900 token_in, 0 + 40 = 40 token_out_1, 0 + 20 = 20 token_out_2
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt(900e9));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount1, BigInt(40e9));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount2, BigInt(20e6));

        // Boss should have: 0 + 100 = 100 token_in
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt(100e9));

        // Vault should have: 1000 - 40 = 960 token_out_1, 1000 - 20 = 980 token_out_2
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount1, BigInt(960e9));
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount2, BigInt(980e6));
    });

    test("Should calculate correct price with fee", async () => {
        const currentTime = await testHelper.getCurrentClockTime();

        await program.makeDualRedemptionOffer({
            startTime: currentTime,
            endTime: currentTime + 86400,
            price1: 1e9,
            price2: 0.5e9,
            ratioBasisPoints: 8000,
            feeBasisPoints: 100,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        const expectedTokenInAmount = 1e9; // 1 ONyc (9 decimals)

        await program.takeDualRedemptionOffer({
            offerId,
            tokenInAmount: expectedTokenInAmount,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2,
            user: user.publicKey,
            signer: user
        });

        // Should receive 80% of 0.99 ONyc in rONYC with price 1.0 = 0.792 USDC
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount1, BigInt(792e6));
        // Should receive 20% of 0.99 ONyc in USDC with price 0.5 = 0.396 rONyc
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount2, BigInt(396e3));
    });


    test("Take dual redemption offer with 0/100 ratio (all to token2) should succeed", async () => {
        const currentTime = await testHelper.getCurrentClockTime();

        // Create dual redemption offer: price1 = 3.0, price2 = 1.5, ratio = 0 (0% for token1, 100% for token2)
        await program.makeDualRedemptionOffer({
            startTime: currentTime,
            endTime: currentTime + 3600,
            price1: 3e9,
            price2: 1.5e9,
            ratioBasisPoints: 0,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // when - user takes offer: pays 30 token_in (9 decimals)
        // Expected calculation:
        // - 0% of 30 = 0 tokens go to token_out_1 → 0 token_out_1
        // - 100% of 30 = 30 tokens go to token_out_2 at price 1.5 → 30/1.5 = 20 token_out_2
        const tokenInAmount = 30e9; // 30 tokens with 9 decimals

        await program.takeDualRedemptionOffer({
            offerId,
            tokenInAmount,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2,
            user: user.publicKey,
            signer: user
        });

        // then - verify balances
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt(970e9)); // 1000 - 30 = 970 (9 decimals)
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount1, BigInt(0)); // Should get 0
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount2, BigInt(20e6)); // Should get 20 tokens (6 decimals)

        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt(30e9)); // Should get 30 (9 decimals)

        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount1, BigInt(1000e9)); // Should remain 1000
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount2, BigInt(980e6)); // 1000 - 20 = 980 (6 decimals)
    });

    test("Take dual redemption offer with 100/0 ratio (all to token1) should succeed", async () => {
        const currentTime = await testHelper.getCurrentClockTime();

        // Create dual redemption offer: price1 = 0.5, price2 = 2.0, ratio = 10000 (100% for token1, 0% for token2)
        await program.makeDualRedemptionOffer({
            startTime: currentTime,
            endTime: currentTime + 3600,
            price1: 0.5e9,
            price2: 2e9,
            ratioBasisPoints: 10000,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // when - user takes offer: pays 10 token_in (9 decimals)
        // Expected calculation:
        // - 100% of 10 = 10 tokens go to token_out_1 at price 0.5 → 10/0.5 = 20 token_out_1
        // - 0% of 10 = 0 tokens go to token_out_2 → 0 token_out_2
        const tokenInAmount = 10e9; // 10 tokens with 9 decimals

        await program.takeDualRedemptionOffer({
            offerId,
            tokenInAmount,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2,
            user: user.publicKey,
            signer: user
        });

        // then - verify balances
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt(990e9)); // 1000 - 10
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount1, BigInt(20e9)); // Should get 20
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount2, BigInt(0)); // Should get 0

        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt(10e9)); // Should get 10

        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount1, BigInt(980e9)); // 1000 - 20 = 980
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount2, BigInt(1000e6)); // Should remain 1000
    });

    test("Take dual redemption offer should fail when offer doesn't exist", async () => {
        const currentTime = await testHelper.getCurrentClockTime();

        // Create dual redemption offer so vault is initialized
        await program.makeDualRedemptionOffer({
            startTime: currentTime,
            endTime: currentTime + 3600,
            price1: 0.5e9,
            price2: 2e9,
            ratioBasisPoints: 10000,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // when/then - try to take non-existent offer
        await expect(
            program.takeDualRedemptionOffer({
                offerId: 9999,
                tokenInAmount: 1000000000,
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2,
                user: user.publicKey,
                signer: user
            })
        ).rejects.toThrow("Offer not found");
    });

    test("Take dual redemption offer should fail when offer expired", async () => {
        const currentTime = await testHelper.getCurrentClockTime();

        await program.makeDualRedemptionOffer({
            startTime: currentTime,
            endTime: currentTime + 3600,
            price1: 1e9,
            price2: 0.5e9,
            ratioBasisPoints: 5000,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        await testHelper.advanceClockBy(7200);

        // when/then - try to take expired offer
        await expect(
            program.takeDualRedemptionOffer({
                offerId,
                tokenInAmount: 1000000000,
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2,
                user: user.publicKey,
                signer: user
            })
        ).rejects.toThrow("Offer has expired");
    });

    test("Take dual redemption offer should fail with wrong token mints", async () => {
        const wrongTokenInMint = testHelper.createMint(9);

        testHelper.createTokenAccount(wrongTokenInMint, user.publicKey, BigInt(100000000000)); // 100 tokens with 9 decimals
        testHelper.createTokenAccount(wrongTokenInMint, boss, BigInt(0));

        const currentTime = await testHelper.getCurrentClockTime();

        await program.makeDualRedemptionOffer({
            startTime: currentTime,
            endTime: currentTime + 3600,
            price1: 1e9,
            price2: 0.5e9,
            ratioBasisPoints: 5000,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // Initialize wrong token mint account manually
        testHelper.createTokenAccount(wrongTokenInMint, program.pdas.dualRedemptionVaultAuthorityPda, BigInt(0), true);

        // when/then - try to take offer with wrong token_in mint
        await expect(
            program.takeDualRedemptionOffer({
                offerId,
                tokenInAmount: 1000000000,
                tokenInMint: wrongTokenInMint,
                tokenOutMint1,
                tokenOutMint2,
                user: user.publicKey,
                signer: user
            })
        ).rejects.toThrow("InvalidTokenInMint");
    });

    describe("Mint/Burn Integration Tests - Take Dual Redemption Offer", () => {

        beforeEach(async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Create redemption offer
            await program.makeDualRedemptionOffer({
                startTime: currentTime,
                endTime: currentTime + 3600,
                price1: 1e9,
                price2: 0.5e9,
                ratioBasisPoints: 5000, // 50% for token_out_1, 50% for token_out_2
                feeBasisPoints: 200, // 2% fee
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2
            });
        });

        describe("Program lacks mint authority tests", () => {
                test("Should transfer token1_out tokens from vault to user when program lacks mint authority", async () => {
                    const tokenInAmount = 100e9;

                    const mintInfoBefore = await testHelper.getMintInfo(tokenInMint);
                    // Token out before
                    const bossTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenOutAccount1);
                    const vaultTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount1);

                    await program.takeDualRedemptionOffer({
                        offerId,
                        tokenInAmount,
                        tokenInMint,
                        tokenOutMint1,
                        tokenOutMint2,
                        user: user.publicKey,
                        signer: user
                    });

                    const mintInfoAfter = await testHelper.getMintInfo(tokenInMint);
                    // Token out after
                    const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount1);
                    const bossTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenOutAccount1);
                    const vaultTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount1);

                    // Verify token changes
                    const supplyBurned = mintInfoBefore.supply - mintInfoAfter.supply;
                    const userReceived = userTokenOutBalance;
                    const vaultDeducted = vaultTokenOutBalanceBefore - vaultTokenOutBalanceAfter;
                    const bossPaid = bossTokenOutBalanceAfter - bossTokenOutBalanceBefore;

                    expect(supplyBurned).toBe(BigInt(0)); // No supply burned
                    expect(userReceived).toBe(BigInt(49e9)); // Should receive 5 token out
                    expect(vaultDeducted).toBe(BigInt(49e9)); // Vault gave token_out
                    expect(bossPaid).toEqual(BigInt(0)); // Boss no change (transferred from vault)
                });

                test("Should transfer token2_out tokens from vault to user when program lacks mint authority", async () => {
                    const tokenInAmount = 100e9;

                    const mintInfoBefore = await testHelper.getMintInfo(tokenOutMint2);
                    // Token out before
                    const bossTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenOutAccount2);
                    const vaultTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount2);

                    await program.takeDualRedemptionOffer({
                        offerId,
                        tokenInAmount,
                        tokenInMint,
                        tokenOutMint1,
                        tokenOutMint2,
                        user: user.publicKey,
                        signer: user
                    });

                    const mintInfoAfter = await testHelper.getMintInfo(tokenOutMint2);
                    // Token out after
                    const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount2);
                    const bossTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenOutAccount2);
                    const vaultTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount2);

                    // Verify token changes
                    const supplyBurned = mintInfoBefore.supply - mintInfoAfter.supply;
                    const userReceived = userTokenOutBalance;
                    const vaultDeducted = vaultTokenOutBalanceBefore - vaultTokenOutBalanceAfter;
                    const bossPaid = bossTokenOutBalanceAfter - bossTokenOutBalanceBefore;

                    expect(supplyBurned).toBe(BigInt(0)); // No supply burned
                    expect(userReceived).toBe(BigInt(98e6)); // Should receive 5 token out
                    expect(vaultDeducted).toBe(BigInt(98e6)); // Vault gave token_out
                    expect(bossPaid).toEqual(BigInt(0)); // Boss no change (transferred from vault)
                });

                it("Should transfer token_in tokens from user to boss when program lacks mint authority", async () => {
                    const tokenInAmount = 100e9;

                    const mintInfoBefore = await testHelper.getMintInfo(tokenInMint);
                    // Token in before
                    const userTokenInBalanceBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
                    const bossTokenInBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                    const vaultTokenInBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenInAccount);

                    // Execute take_dual_redemption_offer without mint authority (should use vault transfer)
                    await program.takeDualRedemptionOffer({
                        offerId,
                        tokenInAmount,
                        tokenInMint,
                        tokenOutMint1,
                        tokenOutMint2,
                        user: user.publicKey,
                        signer: user
                    });

                    const mintInfoAfter = await testHelper.getMintInfo(tokenInMint);
                    // Token in after
                    const userTokenInBalanceAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);
                    const bossTokenInBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                    const vaultTokenInBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenInAccount);

                    // Verify token changes
                    const supplyBurned = mintInfoBefore.supply - mintInfoAfter.supply;
                    const userPaid = userTokenInBalanceBefore - userTokenInBalanceAfter;
                    const vaultReceived = vaultTokenInBalanceAfter - vaultTokenInBalanceBefore;
                    const bossReceived = bossTokenInBalanceAfter - bossTokenInBalanceBefore;

                    expect(supplyBurned).toBe(BigInt(0)); // No supply burned
                    expect(userPaid).toBe(BigInt(tokenInAmount)); // User paid token_in amount
                    expect(vaultReceived).toBe(BigInt(0)); // Vault received no tokens (transferred to boss)
                    expect(bossReceived).toEqual(BigInt(tokenInAmount)); // Boss received token_in tokens
                });
            }
        );

        describe("Program has mint authority tests", () => {
                test("Should mint token1_out tokens directly to user when program has mint authority", async () => {
                    // Transfer mint authority from boss to program for tokenOutMint1
                    await program.transferMintAuthorityToProgram({
                        mint: tokenOutMint1
                    });

                    const tokenInAmount = 100e9;

                    const mintInfoBefore = await testHelper.getMintInfo(tokenOutMint1);
                    // Token out before
                    const bossTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenOutAccount1);
                    const vaultTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount1);

                    await program.takeDualRedemptionOffer({
                        offerId,
                        tokenInAmount,
                        tokenInMint,
                        tokenOutMint1,
                        tokenOutMint2,
                        user: user.publicKey,
                        signer: user
                    });

                    const mintInfoAfter = await testHelper.getMintInfo(tokenOutMint1);
                    // Token out after
                    const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount1);
                    const bossTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenOutAccount1);
                    const vaultTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount1);

                    // Verify token changes
                    const supplyMinted = mintInfoAfter.supply - mintInfoBefore.supply;
                    const userReceived = userTokenOutBalance;
                    const vaultDeducted = vaultTokenOutBalanceBefore - vaultTokenOutBalanceAfter;
                    const bossPaid = bossTokenOutBalanceAfter - bossTokenOutBalanceBefore;

                    expect(supplyMinted).toBe(BigInt(49e9)); // Should mint 49 tokens
                    expect(userReceived).toBe(BigInt(49e9)); // Should receive 49 token out
                    expect(vaultDeducted).toBe(BigInt(0)); // Vault no change (tokens were minted)
                    expect(bossPaid).toEqual(BigInt(0)); // Boss no change (tokens were minted)
                });

                test("Should mint token2_out tokens directly to user when program has mint authority", async () => {
                    // Transfer mint authority from boss to program for tokenOutMint1
                    await program.transferMintAuthorityToProgram({
                        mint: tokenOutMint2
                    });

                    const tokenInAmount = 100e9;

                    const mintInfoBefore = await testHelper.getMintInfo(tokenOutMint2);
                    // Token out before
                    const bossTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenOutAccount2);
                    const vaultTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount2);

                    await program.takeDualRedemptionOffer({
                        offerId,
                        tokenInAmount,
                        tokenInMint,
                        tokenOutMint1,
                        tokenOutMint2,
                        user: user.publicKey,
                        signer: user
                    });

                    const mintInfoAfter = await testHelper.getMintInfo(tokenOutMint2);
                    // Token out after
                    const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount2);
                    const bossTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenOutAccount2);
                    const vaultTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount2);

                    // Verify token changes
                    const supplyMinted = mintInfoAfter.supply - mintInfoBefore.supply;
                    const userReceived = userTokenOutBalance;
                    const vaultDeducted = vaultTokenOutBalanceBefore - vaultTokenOutBalanceAfter;
                    const bossPaid = bossTokenOutBalanceAfter - bossTokenOutBalanceBefore;

                    expect(supplyMinted).toBe(BigInt(98e6)); // Should mint 98 tokens
                    expect(userReceived).toBe(BigInt(98e6)); // Should receive 98 token out
                    expect(vaultDeducted).toBe(BigInt(0)); // Vault no change (tokens were minted)
                    expect(bossPaid).toEqual(BigInt(0)); // Boss no change (tokens were minted)
                });

                test("Should burn token_in tokens when program has mint authority", async () => {
                    // Transfer mint authority from boss to program for tokenInMint
                    await program.transferMintAuthorityToProgram({
                        mint: tokenInMint
                    });

                    const tokenInAmount = 100e9;

                    const mintInfoBefore = await testHelper.getMintInfo(tokenInMint);
                    // Token in before
                    const userTokenInBalanceBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
                    const bossTokenInBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                    const vaultTokenInBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenInAccount);

                    await program.takeDualRedemptionOffer({
                        offerId,
                        tokenInAmount,
                        tokenInMint,
                        tokenOutMint1,
                        tokenOutMint2,
                        user: user.publicKey,
                        signer: user
                    });
                    
                    const mintInfoAfter = await testHelper.getMintInfo(tokenInMint);
                    // Token in after
                    const userTokenInBalanceAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);
                    const bossTokenInBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                    const vaultTokenInBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenInAccount);

                    // Verify token changes
                    const supplyBurned = mintInfoBefore.supply - mintInfoAfter.supply;
                    const userPaid = userTokenInBalanceBefore - userTokenInBalanceAfter;
                    const vaultReceived = vaultTokenInBalanceAfter - vaultTokenInBalanceBefore;
                    const bossReceived = bossTokenInBalanceAfter - bossTokenInBalanceBefore;

                    expect(supplyBurned).toBe(BigInt(tokenInAmount)); // Should burn 100 tokens
                    expect(userPaid).toBe(BigInt(tokenInAmount)); // User paid token_in amount
                    expect(vaultReceived).toBe(BigInt(0)); // Vault received no tokens (burned)
                    expect(bossReceived).toEqual(BigInt(0)); // Boss received no tokens (burned)
                });
            }
        );
    });
});