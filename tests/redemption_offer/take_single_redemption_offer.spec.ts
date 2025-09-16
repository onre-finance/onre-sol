import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { OnreProgram } from "../onre_program.ts";

describe("Take single redemption offer", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    const offerId = 1;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    let boss: PublicKey;
    let user: Keypair;

    let bossTokenInAccount: PublicKey;
    let bossTokenOutAccount: PublicKey;

    let userTokenInAccount: PublicKey;
    let userTokenOutAccount: PublicKey;

    let vaultTokenInAccount: PublicKey;
    let vaultTokenOutAccount: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints
        tokenInMint = testHelper.createMint(9);
        tokenOutMint = testHelper.createMint(6);

        // Initialize program and offers
        await program.initialize();
        await program.initializeOffers();
        await program.initializeVaultAuthority();

        boss = testHelper.getBoss();
        user = testHelper.createUserAccount();

        // Create token accounts
        bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        bossTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, boss, BigInt(10_000e6));

        userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(1000e9)); // 1000 tokens
        userTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, user.publicKey);

        vaultTokenInAccount = testHelper.createTokenAccount(tokenInMint, program.pdas.singleRedemptionVaultAuthorityPda, BigInt(0), true);
        vaultTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, program.pdas.singleRedemptionVaultAuthorityPda, true);

        // Deposit tokens
        await program.singleRedemptionVaultDeposit({ amount: 10_000e6, tokenMint: tokenOutMint }); // 10,000 tokens
    });

    test("Take redemption offer with different decimals (9,6) should succeed", async () => {
        // Create user accounts and fund them
        const vaultTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, program.pdas.singleRedemptionVaultAuthorityPda, true);

        // Create redemption offer: price = 2.0 (2e9 with 9 decimals)
        const startTime = await testHelper.getCurrentClockTime();

        await program.makeSingleRedemptionOffer({
            startTime,
            endTime: startTime + 3600,
            price: 2e9,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint
        });

        // when - user takes offer: pays 10 token_in, should get 5 token_out (10 / 2 = 5)
        const tokenInAmount = 10e9;

        await program.takeSingleRedemptionOffer({
            offerId,
            tokenInAmount,
            tokenInMint,
            tokenOutMint,
            user: user.publicKey,
            signer: user
        });

        // then - verify balances
        // User should have: 1000 - 10 = 990 token_in, 0 + 5 = 5 token_out
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt(990e9));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount, BigInt(5e6));

        // Boss should have: 0 + 10 = 10 token_in
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt(10e9));

        // Vault should have: 10_000 - 5 = 9_995 token_out
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount, BigInt(9995e6));
    });

    test("Should calculate correct price with fee", async () => {
        // Create a new redemption offer: price = 2.0 (with 9 decimals)
        const currentTime = await testHelper.getCurrentClockTime();

        await program.makeSingleRedemptionOffer({
            startTime: currentTime,
            endTime: currentTime + 3600,
            price: 2e9,
            feeBasisPoints: 100,
            tokenInMint,
            tokenOutMint
        });

        // when - user takes offer: pays 10 token_in, pays 1% fee, should get 4.95 token_out (9.9 / 2 = 4.95)
        const tokenInAmount = 10e9; // 10 tokens with 9 decimals

        await program.takeSingleRedemptionOffer({
            offerId,
            tokenInAmount,
            tokenInMint,
            tokenOutMint,
            user: user.publicKey,
            signer: user
        });

        // then - verify balances
        const vaultTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, program.pdas.singleRedemptionVaultAuthorityPda, true);

        // User should have: 1000 - 10 = 990 token_in, 0 + 4.95 = 4.95 token_out
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt(990e9));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount, BigInt(4.95e6));

        // Boss should have: 0 + 10 = 10 token_in
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt(10e9));

        // Vault should have: 10,000 - 4.95 = 9,996.05 token_out
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount, BigInt(9_995.05e6));
    });

    test("Take redemption offer should fail when offer doesn't exist", async () => {
        // when/then - try to take non-existent offer
        await expect(
            program.takeSingleRedemptionOffer({
                offerId: 9999,
                tokenInAmount: 1e9,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            })
        ).rejects.toThrow("Offer not found");
    });

    test("Take redemption offer should fail when offer expired", async () => {
        // given - create expired offer
        const currentTime = await testHelper.getCurrentClockTime();
        const startTime = currentTime - 7200; // 2 hours ago
        const endTime = currentTime - 3600; // 1 hour ago (expired)

        await program.makeSingleRedemptionOffer({
            startTime,
            endTime,
            price: 1e9,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint
        });

        // when/then - try to take expired offer
        await expect(
            program.takeSingleRedemptionOffer({
                offerId,
                tokenInAmount: 1e9,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            })
        ).rejects.toThrow("Offer has expired");
    });

    describe("Burn/Mint Integration Tests", () => {
        beforeEach(async () => {
            // Create redemption offer
            const currentTime = await testHelper.getCurrentClockTime();

            await program.makeSingleRedemptionOffer({
                startTime: currentTime,
                endTime: currentTime + 3600,
                price: 1e9, // 1:1 price
                feeBasisPoints: 100, // 1%
                tokenInMint,
                tokenOutMint
            });
        });

        describe("Program lacks mint authority tests", () => {
            test("Should transfer token_out tokens from vault to user when program lacks mint authority", async () => {
                const tokenInAmount = 100e9; // 100 tokens

                const mintInfoBefore = await testHelper.getMintInfo(tokenOutMint);
                // Token out before
                const bossTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenOutAccount);
                const vaultTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                // Execute take_single_redemption_offer (should transfer to boss, not burn)
                await program.takeSingleRedemptionOffer({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                const mintInfoAfter = await testHelper.getMintInfo(tokenOutMint);
                // Token out after
                const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
                const bossTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenOutAccount);
                const vaultTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                // Verify token changes
                const supplyMinted = mintInfoAfter.supply - mintInfoBefore.supply;
                const userReceived = userTokenOutBalance;
                const vaultDeducted = vaultTokenOutBalanceBefore - vaultTokenOutBalanceAfter;
                const bossPaid = bossTokenOutBalanceAfter - bossTokenOutBalanceBefore;

                expect(supplyMinted).toBe(BigInt(0)); // No supply minted
                expect(userReceived).toBe(BigInt(99e6)); // Should receive 100 token out
                expect(vaultDeducted).toBe(BigInt(99e6)); // Vault gave token_out
                expect(bossPaid).toEqual(BigInt(0)); // Boss no change (transferred from vault)
            });

            it("Should transfer token_in tokens from user to boss when program lacks mint authority", async () => {
                const tokenInAmount = 100e9; // 100 tokens

                const mintInfoBefore = await testHelper.getMintInfo(tokenInMint);
                // Token in before
                const userTokenInBalanceBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
                const bossTokenInBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                const vaultTokenInBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenInAccount);

                // Execute take_buy_offer without mint authority (should use vault transfer)
                await program.takeSingleRedemptionOffer({
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
        });

        describe("Program has mint authority tests", () => {
            test("Should mint token_out tokens when program has mint authority", async () => {
                // Transfer mint authority from boss to program for tokenOutMint
                await program.transferMintAuthorityToProgram({
                    mint: tokenOutMint
                });

                const tokenInAmount = 100e9; // 100 tokens

                const mintInfoBefore = await testHelper.getMintInfo(tokenOutMint);
                // Token out before
                const bossTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenOutAccount);
                const vaultTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                // Execute take_single_redemption_offer (should burn tokens)
                await program.takeSingleRedemptionOffer({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                const mintInfoAfter = await testHelper.getMintInfo(tokenOutMint);
                // Token out after
                const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
                const bossTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(bossTokenOutAccount);
                const vaultTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                // Verify token changes
                const supplyMinted = mintInfoAfter.supply - mintInfoBefore.supply;
                const userReceived = userTokenOutBalance;
                const vaultDeducted = vaultTokenOutBalanceBefore - vaultTokenOutBalanceAfter;
                const bossPaid = bossTokenOutBalanceAfter - bossTokenOutBalanceBefore;

                expect(supplyMinted).toBe(BigInt(99e6)); // 100 tokens minted
                expect(userReceived).toBe(BigInt(99e6)); // Should receive 100 token out
                expect(vaultDeducted).toBe(BigInt(0)); // No change to Vault (tokens were minted)
                expect(bossPaid).toEqual(BigInt(0)); // Boss no change (tokens were minted)
            });

            test("Should burn token_in tokens when program has mint authority", async () => {
                // Transfer mint authority from boss to program for tokenInMint
                await program.transferMintAuthorityToProgram({
                    mint: tokenInMint
                });

                const tokenInAmount = 100e9; // 100 tokens

                const mintInfoBefore = await testHelper.getMintInfo(tokenInMint);
                // Token in before
                const userTokenInBalanceBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
                const bossTokenInBalanceBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                const vaultTokenInBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenInAccount);

                // Execute take_single_redemption_offer (should burn tokens)
                await program.takeSingleRedemptionOffer({
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

                const supplyBurned = mintInfoBefore.supply - mintInfoAfter.supply;
                const userPaid = userTokenInBalanceBefore - userTokenInBalanceAfter;
                const vaultReceived = vaultTokenInBalanceAfter - vaultTokenInBalanceBefore;
                const bossReceived = bossTokenInBalanceAfter - bossTokenInBalanceBefore;

                expect(supplyBurned).toBe(BigInt(tokenInAmount));
                expect(userPaid).toBe(BigInt(tokenInAmount)); // User paid token_in amount
                expect(vaultReceived).toBe(BigInt(0)); // Vault received no tokens (burned)
                expect(bossReceived).toEqual(BigInt(0)); // Boss received no tokens (burned)
            });
        });
    });
});