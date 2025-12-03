import { Keypair, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Fulfill redemption request", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let usdcMint: PublicKey;
    let onycMint: PublicKey;
    let offerPda: PublicKey;
    let redemptionOfferPda: PublicKey;
    let redemptionAdmin: Keypair;
    let redeemer: Keypair;

    const REDEMPTION_AMOUNT = 1_000_000_000; // 1 ONyc (9 decimals)
    const TOKEN_OUT_AMOUNT = 1_000_000; // 1 USDC (6 decimals) at 1:1 price

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints
        usdcMint = testHelper.createMint(6); // USDC (6 decimals)
        onycMint = testHelper.createMint(9); // ONyc (9 decimals)

        // Initialize program
        await program.initialize({ onycMint });

        // Set redemption admin
        redemptionAdmin = testHelper.createUserAccount();
        await program.setRedemptionAdmin({ redemptionAdmin: redemptionAdmin.publicKey });

        // Create user account
        redeemer = testHelper.createUserAccount();

        // Create the base offer (USDC -> ONyc)
        await program.makeOffer({
            tokenInMint: usdcMint,
            tokenOutMint: onycMint
        });

        offerPda = program.getOfferPda(usdcMint, onycMint);

        // Add pricing vector with base price of 1.0
        const currentTime = await testHelper.getCurrentClockTime();
        await program.addOfferVector({
            tokenInMint: usdcMint,
            tokenOutMint: onycMint,
            baseTime: currentTime,
            basePrice: 1e9, // 1.0 (9 decimals)
            apr: 0, // 0% APR for simplicity
            priceFixDuration: 86400 // 1 day
        });

        // Create redemption offer (ONyc -> USDC)
        await program.makeRedemptionOffer({
            offer: offerPda
        });

        redemptionOfferPda = program.getRedemptionOfferPda(onycMint, usdcMint);

        // Fund redeemer with ONyc tokens
        testHelper.createTokenAccount(onycMint, redeemer.publicKey, BigInt(10_000e9), true);
    });

    describe("Basic fulfillment", () => {
        test("Should successfully fulfill a valid redemption request with burn and mint", async () => {
            // given - Transfer mint authority for both tokens to program
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            // Create boss token accounts (needed for the instruction)
            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            // Create redemption request
            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then - Check redemption request status is updated
            const redemptionRequest = await program.getRedemptionRequest(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );
            expect(redemptionRequest.status).toBe(1); // Executed

            // Check user received USDC tokens
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(TOKEN_OUT_AMOUNT));

            // Check ONyc tokens were burned (balance should decrease)
            const userOnycAccount = getAssociatedTokenAddressSync(onycMint, redeemer.publicKey);
            const userOnycBalance = await testHelper.getTokenAccountBalance(userOnycAccount);
            expect(userOnycBalance).toBe(BigInt(10_000e9 - REDEMPTION_AMOUNT));
        });

        test("Should transfer token_in to boss when program lacks mint authority", async () => {
            // given - Program does NOT have mint authority for ONyc
            // Transfer USDC mint authority to program for minting token_out
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            // Create boss token accounts
            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            // Create redemption request
            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then - Check boss received ONyc tokens
            const bossOnycAccount = getAssociatedTokenAddressSync(onycMint, boss);
            const bossOnycBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(bossOnycBalance).toBe(BigInt(REDEMPTION_AMOUNT));
        });

        test("Should transfer token_out from vault when program lacks mint authority", async () => {
            // given - Program does NOT have mint authority for USDC
            // Transfer ONyc mint authority to program for burning token_in
            await program.transferMintAuthorityToProgram({ mint: onycMint });

            // Create boss token accounts and fund with USDC for vault deposit
            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(10_000e6), true); // Fund boss with USDC

            // Deposit USDC to redemption vault (boss deposits)
            await program.redemptionVaultDeposit({
                amount: 10_000e6,
                tokenMint: usdcMint
            });

            // Create redemption request
            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then - Check USDC transferred from vault
            const vaultUsdcAccount = getAssociatedTokenAddressSync(
                usdcMint,
                program.pdas.redemptionVaultAuthorityPda,
                true
            );
            const vaultBalance = await testHelper.getTokenAccountBalance(vaultUsdcAccount);
            expect(vaultBalance).toBe(BigInt(10_000e6 - TOKEN_OUT_AMOUNT));
        });
    });

    describe("Redemption statistics", () => {
        test("Should update executed_redemptions in RedemptionOffer", async () => {
            // given
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            // Create boss token accounts
            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then
            const redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
            expect(redemptionOffer.executedRedemptions.toString()).toBe(
                REDEMPTION_AMOUNT.toString()
            );
        });

        test("Should decrement requested_redemptions in RedemptionOffer", async () => {
            // given
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            // Create boss token accounts
            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce
            });

            // Verify requested_redemptions increased
            let redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
            expect(redemptionOffer.requestedRedemptions.toString()).toBe(
                REDEMPTION_AMOUNT.toString()
            );

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then
            redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
            expect(redemptionOffer.requestedRedemptions.toString()).toBe("0");
        });

        test("Should accumulate executed_redemptions from multiple fulfillments", async () => {
            // given
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            // Create boss token accounts
            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const redeemer2 = testHelper.createUserAccount();
            testHelper.createTokenAccount(onycMint, redeemer2.publicKey, BigInt(10_000e9), true);

            // Create two redemption requests
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce: 0
            });

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer: redeemer2,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT * 2,
                expiresAt,
                nonce: 0
            });

            // when - Fulfill both requests
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: program.getRedemptionRequestPda(
                    redemptionOfferPda,
                    redeemer.publicKey,
                    0
                ),
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: program.getRedemptionRequestPda(
                    redemptionOfferPda,
                    redeemer2.publicKey,
                    0
                ),
                redeemer: redeemer2.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then
            const redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
            expect(redemptionOffer.executedRedemptions.toString()).toBe(
                (REDEMPTION_AMOUNT * 3).toString()
            );
        });
    });

    describe("Access control", () => {
        test("Should reject when redemption_admin is not authorized", async () => {
            // given
            const unauthorizedAdmin = testHelper.createUserAccount();

            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when/then
            await expect(
                program.fulfillRedemptionRequest({
                    offer: offerPda,
                    redemptionOffer: redemptionOfferPda,
                    redemptionRequest: redemptionRequestPda,
                    redeemer: redeemer.publicKey,
                    redemptionAdmin: unauthorizedAdmin, // Wrong admin
                    tokenInMint: onycMint,
                    tokenOutMint: usdcMint
                })
            ).rejects.toThrow();
        });

        test("Should reject when kill switch is activated", async () => {
            // given
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            // Create boss token accounts
            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // Activate kill switch
            await program.setKillSwitch({ enable: true });

            // when/then
            await expect(
                program.fulfillRedemptionRequest({
                    offer: offerPda,
                    redemptionOffer: redemptionOfferPda,
                    redemptionRequest: redemptionRequestPda,
                    redeemer: redeemer.publicKey,
                    redemptionAdmin,
                    tokenInMint: onycMint,
                    tokenOutMint: usdcMint
                })
            ).rejects.toThrow();
        });

        test("Should reject when request has already been fulfilled", async () => {
            // given
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            // Create boss token accounts
            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // Fulfill once
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // when/then - Try to fulfill again
            await expect(
                program.fulfillRedemptionRequest({
                    offer: offerPda,
                    redemptionOffer: redemptionOfferPda,
                    redemptionRequest: redemptionRequestPda,
                    redeemer: redeemer.publicKey,
                    redemptionAdmin,
                    tokenInMint: onycMint,
                    tokenOutMint: usdcMint
                })
            ).rejects.toThrow();
        });

        // Note: Testing expiration is challenging in bankrun since:
        // 1. create_redemption_request validates expiration at creation time
        // 2. Bankrun doesn't easily support time manipulation
        // Expiration is validated at both creation and fulfillment time in production
    });

    describe("Price calculation", () => {
        test("Should use current price from offer for redemption", async () => {
            // given - Create offer with price 2.0 (user pays 2 ONyc to get 1 USDC back)
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            // Create boss token accounts
            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const currentTime = await testHelper.getCurrentClockTime();

            // Add new vector with price 2.0 (will be the most recent active vector)
            // Use a unique timestamp to avoid duplicate start_time error
            await program.addOfferVector({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                baseTime: currentTime + 100, // Start 100 seconds in the future
                basePrice: 2e9, // 2.0 (9 decimals) - 2 USDC per 1 ONyc
                apr: 0,
                priceFixDuration: 86400
            });

            // Advance clock so the new vector becomes active
            await testHelper.advanceClockBy(100);

            // Create redemption request for 2 ONyc
            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;
            const amountIn = 2_000_000_000; // 2 ONyc

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: amountIn,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then - User should receive 4 USDC (2 ONyc × 2.0 USDC/ONyc = 4 USDC)
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(4_000_000)); // 4 USDC
        });

        test("Should handle fractional price 1.003 correctly", async () => {
            // given - Price of 1.003 USDC per ONyc
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const currentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                baseTime: currentTime + 200,
                basePrice: 1_003_000_000, // 1.003 USDC per ONyc (9 decimals)
                apr: 0,
                priceFixDuration: 86400
            });
            await testHelper.advanceClockBy(200);

            // Redeem 10 ONyc -> should get 10 * 1.003 = 10.03 USDC
            const amountIn = 10_000_000_000; // 10 ONyc (9 decimals)
            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: amountIn,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then - Should receive 10.03 USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(10_030_000)); // 10.03 USDC (6 decimals)
        });

        test("Should handle tokens with different decimals (9 vs 6)", async () => {
            // given - ONyc has 9 decimals, USDC has 6 decimals, price 0.5
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const currentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                baseTime: currentTime + 300,
                basePrice: 500_000_000, // 0.5 USDC per ONyc
                apr: 0,
                priceFixDuration: 86400
            });
            await testHelper.advanceClockBy(300);

            // Redeem 100 ONyc -> should get 100 * 0.5 = 50 USDC
            const amountIn = 100_000_000_000; // 100 ONyc (9 decimals)
            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: amountIn,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then - Should receive 50 USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(50_000_000)); // 50 USDC (6 decimals)
        });

        test("Should handle high precision price 3.141592653", async () => {
            // given - Price of π (pi) approximation
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const currentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                baseTime: currentTime + 400,
                basePrice: 3_141_592_653, // π ≈ 3.141592653 USDC per ONyc (9 decimals)
                apr: 0,
                priceFixDuration: 86400
            });
            await testHelper.advanceClockBy(400);

            // Redeem 7 ONyc -> should get 7 * 3.141592653 = 21.991148571 USDC
            const amountIn = 7_000_000_000; // 7 ONyc (9 decimals)
            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: amountIn,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then - Should receive 21.991148 USDC (truncated to 6 decimals)
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(21_991_148)); // 21.991148 USDC (6 decimals, truncated)
        });

        test("Should handle very small amounts correctly", async () => {
            // given - Price of 100 USDC per ONyc
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const currentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                baseTime: currentTime + 500,
                basePrice: 100_000_000_000, // 100 USDC per ONyc (9 decimals)
                apr: 0,
                priceFixDuration: 86400
            });
            await testHelper.advanceClockBy(500);

            // Redeem 0.001 ONyc -> should get 0.001 * 100 = 0.1 USDC
            const amountIn = 1_000_000; // 0.001 ONyc (9 decimals)
            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: amountIn,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then - Should receive 0.1 USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(100_000)); // 0.1 USDC (6 decimals)
        });

        test("Should handle price with many decimal places 0.123456789", async () => {
            // given - Very precise fractional price
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const currentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                baseTime: currentTime + 600,
                basePrice: 123_456_789, // 0.123456789 USDC per ONyc (9 decimals)
                apr: 0,
                priceFixDuration: 86400
            });
            await testHelper.advanceClockBy(600);

            // Redeem 1000 ONyc -> should get 1000 * 0.123456789 = 123.456789 USDC
            const amountIn = 1000_000_000_000; // 1000 ONyc (9 decimals)
            const nonce = 0;
            const expiresAt = Math.floor(Date.now() / 1000) + 3600;

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: amountIn,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then - Should receive 123.456789 USDC (truncated to 6 decimals)
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(123_456_789)); // 123.456789 USDC (6 decimals)
        });

        test("Should handle APR-based price growth correctly", async () => {
            // given - Base price 1.0 with 3.65% APR
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            const currentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                baseTime: currentTime + 700,
                basePrice: 1_000_000_000, // 1.0 USDC per ONyc (9 decimals)
                apr: 36_500, // 3.65% APR (scaled by 1M)
                priceFixDuration: 86400 // 1 day intervals
            });

            // Advance to first interval (1 day)
            // With discrete intervals, price snaps to end of interval
            // At 1 day: interval_end = 2 * 86400, Price = 1.0 * (1 + 0.0365 * 2/365) = 1.0002
            await testHelper.advanceClockBy(700 + 86400);

            // Redeem 100 ONyc -> should get 100 * 1.0002 = 100.02 USDC
            const amountIn = 100_000_000_000; // 100 ONyc (9 decimals)
            const nonce = 0;
            const currentTimeAfterAdvance = await testHelper.getCurrentClockTime();
            const expiresAt = currentTimeAfterAdvance + 3600;

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: amountIn,
                expiresAt,
                nonce
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                redeemer.publicKey,
                nonce
            );

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint
            });

            // then - Should receive 100.02 USDC (price grew with APR)
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(100_020_000)); // 100.02 USDC (6 decimals)
        });
    });
});
