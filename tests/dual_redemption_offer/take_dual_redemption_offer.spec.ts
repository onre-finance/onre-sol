import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

describe("Take dual redemption offer", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

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
        userTokenOutAccount1 = testHelper.createTokenAccount(tokenOutMint1, user.publicKey, BigInt(0));
        userTokenOutAccount2 = testHelper.createTokenAccount(tokenOutMint2, user.publicKey, BigInt(0));

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
        // Create dual redemption offer: price1 = 2.0, price2 = 1.0, ratio = 8000 (80% for token1, 20% for token2)
        const price1 = 2000000000; // 2.0 * 10^9
        const price2 = 1000000000; // 1.0 * 10^9
        const ratioBasisPoints = 8000; // 80% for token_out_1
        const startTime = Math.floor(Date.now() / 1000) - 60; // Start 1 minute ago
        const endTime = startTime + 3600;

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // Get offer ID
        const dualRedemptionOfferAccountData = await program.getDualRedemptionOfferAccount();
        const offerId = dualRedemptionOfferAccountData.counter.toNumber();

        // when - user takes offer: pays 100 token_in
        // Expected calculation:
        // - 80% of 100 = 80 tokens go to token_out_1 at price 2.0 → 80/2 = 40 token_out_1
        // - 20% of 100 = 20 tokens go to token_out_2 at price 1.0 → 20/1 = 20 token_out_2
        const tokenInAmount = 100000000000; // 100 tokens with 9 decimals

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
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt("900000000000"));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount1, BigInt("40000000000"));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount2, BigInt("20000000"));

        // Boss should have: 0 + 100 = 100 token_in
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt("100000000000"));

        // Vault should have: 1000 - 40 = 960 token_out_1, 1000 - 20 = 980 token_out_2
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount1, BigInt("960000000000"));
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount2, BigInt("980000000"));
    });

    test("Should calculate correct price with fee", async () => {
        const currentTime = await testHelper.getCurrentClockTime();

        const offerId = 1;
        const startTime = currentTime;
        const endTime = startTime + 86400;
        const price1 = 1_000_000_000; // 1.0 ONyc for 1 USDC
        const price2 = 500_000_000; // 0.5 ONyc per rONyc
        const ratioBasisPoints = 8000; // 80/20 split
        const feeBasisPoints = 100; // 1% fee

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints,
            feeBasisPoints,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        const expectedTokenInAmount = 1_000_000_000; // 1 ONyc (9 decimals)

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
        // Create dual redemption offer: price1 = 3.0, price2 = 1.5, ratio = 0 (0% for token1, 100% for token2)
        const price1 = 3000000000; // 3.0 * 10^9
        const price2 = 1500000000; // 1.5 * 10^9
        const ratioBasisPoints = 0; // 0% for token_out_1, 100% for token_out_2
        const startTime = Math.floor(Date.now() / 1000) - 60;
        const endTime = startTime + 3600;

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // Get offer ID
        const dualRedemptionOfferAccountData = await program.getDualRedemptionOfferAccount();
        const offerId = dualRedemptionOfferAccountData.counter.toNumber();

        // when - user takes offer: pays 30 token_in (9 decimals)
        // Expected calculation:
        // - 0% of 30 = 0 tokens go to token_out_1 → 0 token_out_1
        // - 100% of 30 = 30 tokens go to token_out_2 at price 1.5 → 30/1.5 = 20 token_out_2
        const tokenInAmount = 30000000000; // 30 tokens with 9 decimals

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
        // Create dual redemption offer: price1 = 0.5, price2 = 2.0, ratio = 10000 (100% for token1, 0% for token2)
        const price1 = 500000000; // 0.5 * 10^9
        const price2 = 2000000000; // 2.0 * 10^9
        const ratioBasisPoints = 10000; // 100% for token_out_1, 0% for token_out_2
        const startTime = Math.floor(Date.now() / 1000) - 60;
        const endTime = startTime + 3600;

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // Get offer ID
        const dualRedemptionOfferAccountData = await program.getDualRedemptionOfferAccount();
        const offerId = dualRedemptionOfferAccountData.counter.toNumber();

        // when - user takes offer: pays 10 token_in (9 decimals)
        // Expected calculation:
        // - 100% of 10 = 10 tokens go to token_out_1 at price 0.5 → 10/0.5 = 20 token_out_1
        // - 0% of 10 = 0 tokens go to token_out_2 → 0 token_out_2
        const tokenInAmount = 10000000000; // 10 tokens with 9 decimals

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
        // Create dual redemption offer so vault is initialized
        const price1 = 500000000; // 0.5 * 10^9
        const price2 = 2000000000; // 2.0 * 10^9
        const ratioBasisPoints = 10000; // 100% for token_out_1, 0% for token_out_2
        const startTime = Math.floor(Date.now() / 1000) - 60;
        const endTime = startTime + 3600;

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints,
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
        const price1 = 1000000000;
        const price2 = 500000000;
        const currentTime = Math.floor(Date.now() / 1000);
        const startTime = currentTime - 7200; // 2 hours ago
        const endTime = currentTime - 3600; // 1 hour ago (expired)

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints: 5000,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        const dualRedemptionOfferAccountData = await program.getDualRedemptionOfferAccount();
        const offerId = dualRedemptionOfferAccountData.counter.toNumber();

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

        const price1 = 1000000000;
        const price2 = 500000000;
        const startTime = Math.floor(Date.now() / 1000) - 60;
        const endTime = startTime + 3600;

        await program.makeDualRedemptionOffer({
            startTime,
            endTime,
            price1,
            price2,
            ratioBasisPoints: 5000,
            feeBasisPoints: 0,
            tokenInMint,
            tokenOutMint1,
            tokenOutMint2
        });

        // Initialize wrong token mint account manually
        testHelper.createTokenAccount(wrongTokenInMint, program.pdas.dualRedemptionVaultAuthorityPda, BigInt(0), true);

        const dualRedemptionOfferAccountData = await program.getDualRedemptionOfferAccount();
        const offerId = dualRedemptionOfferAccountData.counter.toNumber();

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
});