import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { OnreProgram } from "../onre_program.ts";

describe("Take single redemption offer", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    let boss: PublicKey;
    let user: Keypair;

    let bossTokenInAccount: PublicKey;
    let bossTokenOutAccount: PublicKey;

    let userTokenInAccount: PublicKey;
    let userTokenOutAccount: PublicKey;

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
        bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(10_000e9));
        bossTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, boss, BigInt(10_000e6));

        userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(1000e9)); // 1000 tokens
        userTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, user.publicKey);

        // Deposit tokens
        await program.singleRedemptionVaultDeposit({ amount: 10_000e9, tokenMint: tokenInMint }); // 10,000 tokens
        await program.singleRedemptionVaultDeposit({ amount: 10_000e6, tokenMint: tokenOutMint }); // 10,000 tokens
    });

    test("Take redemption offer with same decimals (9,6) should succeed", async () => {
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

        const offerId = 1;

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
        const offerId = 1;
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

        const offerId = 1;

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

    describe("Burn/Transfer Integration Tests", () => {
        describe("Token Transfer (Fallback) Scenarios", () => {
            test("Should successfully transfer tokens to boss when program lacks mint authority", async () => {
                // Create redemption offer
                const currentTime = await testHelper.getCurrentClockTime();

                await program.makeSingleRedemptionOffer({
                    startTime: currentTime,
                    endTime: currentTime + 3600,
                    price: 1e9, // 1:1 price
                    feeBasisPoints: 0,
                    tokenInMint,
                    tokenOutMint
                });

                const offerId = 1;
                const tokenInAmount = 100e9; // 100 tokens
                const bossInBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);

                // Execute take_single_redemption_offer (should transfer to boss, not burn)
                await program.takeSingleRedemptionOffer({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                // Verify token_in was transferred to boss (not burned)
                const userInAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);
                const bossInAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);

                const bossReceived = bossInAfter - bossInBefore;

                expect(bossReceived).toEqual(BigInt(100e9)); // Boss received tokens
                expect(userInAfter).toEqual(BigInt(900e9)); // User paid tokens
            });
        });

        describe("Token Burning Scenarios", () => {
            beforeEach(async () => {
                // Transfer mint authority from boss to program for burnTokenInMint
                await program.transferMintAuthorityToProgram({ mint: tokenInMint });
            });

            test("Should successfully burn tokens when program has mint authority", async () => {
                // Create redemption offer
                const currentTime = await testHelper.getCurrentClockTime();

                await program.makeSingleRedemptionOffer({
                    startTime: currentTime,
                    endTime: currentTime + 3600,
                    price: 1e9, // 1:1 price
                    feeBasisPoints: 0,
                    tokenInMint: tokenInMint,
                    tokenOutMint: tokenOutMint
                });

                const offerId = 1;
                const tokenInAmount = 100e9; // 100 tokens

                // Get initial mint supply
                const mintInfoBefore = await testHelper.getMintInfo(tokenInMint);
                const userInBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
                const bossBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);

                // Execute take_single_redemption_offer (should burn tokens)
                await program.takeSingleRedemptionOffer({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                // Verify tokens were burned (supply reduced)
                const mintInfoAfter = await testHelper.getMintInfo(tokenInMint);
                const userInAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);
                const bossAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);

                const supplyBurned = mintInfoBefore.supply - mintInfoAfter.supply;
                const userPaid = userInBefore - userInAfter;
                const bossReceived = bossAfter - bossBefore;

                expect(supplyBurned).toEqual(BigInt(100e9)); // Tokens were burned
                expect(userPaid).toEqual(BigInt(100e9)); // User paid tokens
                expect(bossReceived).toEqual(BigInt(0)); // Boss received nothing (tokens burned)

                // User received correct amount of token_out
                const userOutAfter = await testHelper.getTokenAccountBalance(testHelper.getAssociatedTokenAccount(tokenOutMint, user.publicKey));
                expect(userOutAfter).toEqual(BigInt(100e6)); // 1:1 exchange
            });

            test("Should handle fee calculations correctly when burning", async () => {
                // Create redemption offer with 5% fee
                const currentTime = await testHelper.getCurrentClockTime();

                await program.makeSingleRedemptionOffer({
                    startTime: currentTime,
                    endTime: currentTime + 3600,
                    price: 1e9, // 1:1 price
                    feeBasisPoints: 500, // 5% fee
                    tokenInMint,
                    tokenOutMint
                });

                const offerId = 1;
                const tokenInAmount = 100e9; // 100 tokens

                // Get initial mint supply
                const mintInfoBefore = await testHelper.getMintInfo(tokenInMint);
                const userInBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
                const bossBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);

                // Execute take_single_redemption_offer
                await program.takeSingleRedemptionOffer({
                    offerId,
                    tokenInAmount,
                    tokenInMint,
                    tokenOutMint,
                    user: user.publicKey,
                    signer: user
                });

                // Verify all 100 tokens were burned (full amount including fee)
                const mintInfoAfter = await testHelper.getMintInfo(tokenInMint);
                const userInAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);
                const bossAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);

                const supplyBurned = mintInfoBefore.supply - mintInfoAfter.supply;
                const userPaid = userInBefore - userInAfter;
                const bossReceived = bossAfter - bossBefore;

                expect(supplyBurned).toEqual(BigInt(100e9)); // Full amount burned
                expect(userPaid).toEqual(BigInt(100e9)); // User paid full amount
                expect(bossReceived).toEqual(BigInt(0)); // Boss received nothing (tokens burned)

                // User received tokens based on amount after fee: 95 tokens out
                const userOutAfter = await testHelper.getTokenAccountBalance(testHelper.getAssociatedTokenAccount(tokenOutMint, user.publicKey));
                expect(userOutAfter).toEqual(BigInt(95e6)); // 95 tokens (after 5% fee)
            });
        });
    });
});