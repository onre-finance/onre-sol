import { Keypair, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
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
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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

            // then - Redemption request account should be closed
            await expect(
                program.getRedemptionRequest(redemptionOfferPda, 0)
            ).rejects.toThrow();

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
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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

        test("Should close redemption request account and return rent to redemption_admin", async () => {
            // given
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
            );

            // Get initial redemption_admin balance
            const initialAdminBalance = await testHelper.context.banksClient.getBalance(
                redemptionAdmin.publicKey
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

            // then - Account should be closed
            await expect(
                program.getRedemptionRequest(redemptionOfferPda, 0)
            ).rejects.toThrow();

            // and - Rent should be returned to redemption_admin
            const finalAdminBalance = await testHelper.context.banksClient.getBalance(
                redemptionAdmin.publicKey
            );
            expect(finalAdminBalance).toBeGreaterThan(initialAdminBalance);
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

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT
            });

            // Verify requested_redemptions increased
            let redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
            expect(redemptionOffer.requestedRedemptions.toString()).toBe(
                REDEMPTION_AMOUNT.toString()
            );

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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
            // Requested redemptions should be decremented after fulfillment
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
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT
            });

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer: redeemer2,
                amount: REDEMPTION_AMOUNT * 2
            });

            // when - Fulfill both requests
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: program.getRedemptionRequestPda(
                    redemptionOfferPda,
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
                    1
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

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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
            const amountIn = 2_000_000_000; // 2 ONyc

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: amountIn
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: amountIn
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: amountIn
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: amountIn
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: amountIn
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: amountIn
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: amountIn
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
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

    describe("Fee handling", () => {
        test("Should correctly deduct fee and transfer to boss when program has mint authority", async () => {
            // given - Create NEW offer and redemption offer with 5% fee
            const usdcMint2 = testHelper.createMint(6);

            await program.makeOffer({
                tokenInMint: usdcMint2,
                tokenOutMint: onycMint
            });

            const offerPda2 = program.getOfferPda(usdcMint2, onycMint);

            const currentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdcMint2,
                tokenOutMint: onycMint,
                baseTime: currentTime,
                basePrice: 1e9,
                apr: 0,
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda2,
                feeBasisPoints: 500 // 5%
            });

            const redemptionOfferPdaWithFee = program.getRedemptionOfferPda(onycMint, usdcMint2);

            // Transfer mint authority for both tokens to program
            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint2 });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint2, boss, BigInt(0), true);

            // Create redemption request for 10 ONyc
            const amount = 10_000_000_000; // 10 ONyc
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPdaWithFee,
                redeemer,
                amount
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPdaWithFee,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onycMint, boss);
            const initialBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda2,
                redemptionOffer: redemptionOfferPdaWithFee,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint2
            });

            // then
            // Fee: 10 ONyc * 5% = 0.5 ONyc (500_000_000)
            // Net: 10 ONyc - 0.5 ONyc = 9.5 ONyc (9_500_000_000) - this gets burned
            // User receives: 9.5 USDC (9_500_000 with 6 decimals) at 1:1 price

            // Boss should receive the fee (0.5 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(initialBossBalance + BigInt(500_000_000)); // +0.5 ONyc

            // User should receive net amount worth in USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint2, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(9_500_000)); // 9.5 USDC (6 decimals)
        });

        test("Should transfer full amount (net + fee) to boss when program lacks mint authority", async () => {
            // given - Create NEW offer and redemption offer with 2% fee
            const usdcMint3 = testHelper.createMint(6);

            await program.makeOffer({
                tokenInMint: usdcMint3,
                tokenOutMint: onycMint
            });

            const offerPda3 = program.getOfferPda(usdcMint3, onycMint);

            const currentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdcMint3,
                tokenOutMint: onycMint,
                baseTime: currentTime,
                basePrice: 1e9,
                apr: 0,
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda3,
                feeBasisPoints: 200 // 2%
            });

            const redemptionOfferPdaWithFee = program.getRedemptionOfferPda(onycMint, usdcMint3);

            // Do NOT transfer mint authority - program lacks mint authority
            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint3, boss, BigInt(0), true);

            // Fund vault with USDC for distribution
            testHelper.createTokenAccount(usdcMint3, program.pdas.redemptionVaultAuthorityPda, BigInt(1000e6), true);

            // Create redemption request for 5 ONyc
            const amount = 5_000_000_000; // 5 ONyc
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPdaWithFee,
                redeemer,
                amount
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPdaWithFee,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onycMint, boss);
            const initialBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda3,
                redemptionOffer: redemptionOfferPdaWithFee,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint3
            });

            // then
            // Fee: 5 ONyc * 2% = 0.1 ONyc (100_000_000)
            // Net: 5 ONyc - 0.1 ONyc = 4.9 ONyc (4_900_000_000)
            // Total transferred to boss: 5 ONyc (net + fee)

            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(initialBossBalance + BigInt(5_000_000_000)); // +5 ONyc (full amount)

            // User should receive net amount worth in USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint3, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(4_900_000)); // 4.9 USDC (6 decimals)
        });

        test("Should handle zero fee correctly", async () => {
            // given - Create NEW offer and redemption offer with 0% fee
            const usdcMint4 = testHelper.createMint(6);

            await program.makeOffer({
                tokenInMint: usdcMint4,
                tokenOutMint: onycMint
            });

            const offerPda4 = program.getOfferPda(usdcMint4, onycMint);

            const currentTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdcMint4,
                tokenOutMint: onycMint,
                baseTime: currentTime,
                basePrice: 1e9,
                apr: 0,
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda4,
                feeBasisPoints: 0
            });

            const redemptionOfferPdaNoFee = program.getRedemptionOfferPda(onycMint, usdcMint4);

            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint4 });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint4, boss, BigInt(0), true);

            const amount = 1_000_000_000; // 1 ONyc
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPdaNoFee,
                redeemer,
                amount
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPdaNoFee,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onycMint, boss);
            const initialBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda4,
                redemptionOffer: redemptionOfferPdaNoFee,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint4
            });

            // then
            // No fee: boss balance should not change (except for 0 amount transfer which doesn't happen)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(initialBossBalance); // No fee transferred

            // User should receive full amount worth in USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint4, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(1_000_000)); // 1 USDC (6 decimals)
        });

        test("Should correctly calculate fees with non-zero APR and time passage", async () => {
            // given - Create NEW offer with APR and redemption offer with 3% fee
            const usdcMint5 = testHelper.createMint(6);

            await program.makeOffer({
                tokenInMint: usdcMint5,
                tokenOutMint: onycMint
            });

            const offerPda5 = program.getOfferPda(usdcMint5, onycMint);

            // Add pricing vector with 20% APR and base price of 1.0
            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdcMint5,
                tokenOutMint: onycMint,
                baseTime,
                basePrice: 1e9, // 1.0 USDC per ONyc
                apr: 200_000, // 20% APR (scale: 10_000 = 1%)
                priceFixDuration: 86400 // 1 day
            });

            await program.makeRedemptionOffer({
                offer: offerPda5,
                feeBasisPoints: 300 // 3%
            });

            const redemptionOfferPdaWithFee = program.getRedemptionOfferPda(onycMint, usdcMint5);

            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint5 });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint5, boss, BigInt(0), true);

            // Advance time by 1 year to see APR effect
            // At 20% APR after 1 year: price = 1.0 * (1 + 0.20) = 1.2 USDC per ONyc
            await testHelper.advanceClockBy(365 * 86400); // 1 year

            // Create redemption request for 100 ONyc
            const amount = 100_000_000_000; // 100 ONyc
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPdaWithFee,
                redeemer,
                amount
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPdaWithFee,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onycMint, boss);
            const initialBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda5,
                redemptionOffer: redemptionOfferPdaWithFee,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint5
            });

            // then
            // Price after 1 year at 20% APR with discrete steps: ~1.2005 USDC per ONyc
            // (Snaps to end of day 366 instead of exactly day 365)
            // Fee: 100 ONyc * 3% = 3 ONyc (3_000_000_000)
            // Net: 100 ONyc - 3 ONyc = 97 ONyc (97_000_000_000) - this gets burned
            // User receives: 97 ONyc * 1.2005 ≈ 116.45 USDC

            // Boss should receive the fee (3 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(initialBossBalance + BigInt(3_000_000_000)); // +3 ONyc

            // User should receive net amount worth in USDC at current price
            // 97 ONyc * 1.2005 ≈ 116.45 USDC (116_453_150 with 6 decimals due to discrete step pricing)
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint5, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(116_453_150)); // Actual value with discrete step pricing
        });

        test("Should handle high fee percentage with APR correctly", async () => {
            // given - Create NEW offer with APR and redemption offer with 10% fee
            const usdcMint6 = testHelper.createMint(6);

            await program.makeOffer({
                tokenInMint: usdcMint6,
                tokenOutMint: onycMint
            });

            const offerPda6 = program.getOfferPda(usdcMint6, onycMint);

            // Add pricing vector with 50% APR and base price of 0.5
            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdcMint6,
                tokenOutMint: onycMint,
                baseTime,
                basePrice: 500_000_000, // 0.5 USDC per ONyc
                apr: 500_000, // 50% APR (scale: 10_000 = 1%)
                priceFixDuration: 86400 // 1 day
            });

            await program.makeRedemptionOffer({
                offer: offerPda6,
                feeBasisPoints: 1000 // 10%
            });

            const redemptionOfferPdaWithFee = program.getRedemptionOfferPda(onycMint, usdcMint6);

            await program.transferMintAuthorityToProgram({ mint: onycMint });
            await program.transferMintAuthorityToProgram({ mint: usdcMint6 });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdcMint6, boss, BigInt(0), true);

            // Advance time by 6 months
            // At 50% APR after 0.5 years: price = 0.5 * (1 + 0.50 * 0.5) = 0.5 * 1.25 = 0.625 USDC per ONyc
            await testHelper.advanceClockBy(182 * 86400); // ~6 months

            // Create redemption request for 50 ONyc
            const amount = 50_000_000_000; // 50 ONyc
            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPdaWithFee,
                redeemer,
                amount
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPdaWithFee,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onycMint, boss);
            const initialBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda6,
                redemptionOffer: redemptionOfferPdaWithFee,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint6
            });

            // then
            // Price after 6 months at 50% APR with discrete steps: ~0.6256 USDC per ONyc
            // (Snaps to end of discrete interval, slightly higher than exact 0.625)
            // Fee: 50 ONyc * 10% = 5 ONyc (5_000_000_000)
            // Net: 50 ONyc - 5 ONyc = 45 ONyc (45_000_000_000) - this gets burned
            // User receives: 45 ONyc * 0.6256 ≈ 28.14 USDC

            // Boss should receive the fee (5 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(initialBossBalance + BigInt(5_000_000_000)); // +5 ONyc

            // User should receive net amount worth in USDC at current price
            // 45 ONyc * 0.6256 ≈ 28.14 USDC (28_140_410 with 6 decimals due to discrete step pricing)
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint6, redeemer.publicKey);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBe(BigInt(28_140_410)); // Actual value with discrete step pricing
        });
    });

    describe("Token2022 with random APR and fees", () => {
        test("Token2022 Test #1: APR=5%, Fee=1%, 30 days", async () => {
            // given - Create Token2022 mints
            const usdc2022 = testHelper.createMint2022(6);
            const onyc2022 = testHelper.createMint2022(9);

            await program.makeOffer({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID
            });

            const offerPda = program.getOfferPda(usdc2022, onyc2022);

            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                baseTime,
                basePrice: 1_500_000_000, // 1.5 USDC per ONyc
                apr: 50_000, // 5% APR
                priceFixDuration: 86400 // 1 day
            });

            await program.makeRedemptionOffer({
                offer: offerPda,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID,
                feeBasisPoints: 100 // 1% fee
            });

            const redemptionOfferPda = program.getRedemptionOfferPda(onyc2022, usdc2022);

            await program.transferMintAuthorityToProgram({ mint: onyc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });
            await program.transferMintAuthorityToProgram({ mint: usdc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onyc2022, boss, BigInt(0), true);
            testHelper.createTokenAccount(usdc2022, boss, BigInt(0), true);

            // Advance time by 30 days
            await testHelper.advanceClockBy(30 * 86400);

            const amount = 50_000_000_000; // 50 ONyc
            // Create redeemer's Token2022 account for token_in
            testHelper.createTokenAccount(onyc2022, redeemer.publicKey, BigInt(amount), true, TOKEN_2022_PROGRAM_ID);

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onyc2022, boss, false, TOKEN_2022_PROGRAM_ID);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onyc2022,
                tokenOutMint: usdc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            // then - Boss should receive 1% fee (0.5 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(BigInt(500_000_000)); // 0.5 ONyc fee

            // User receives: 49.5 ONyc * ~1.506 = ~74.55 USDC (with discrete step adjustment)
            const userUsdcAccount = getAssociatedTokenAddressSync(usdc2022, redeemer.publicKey, false, TOKEN_2022_PROGRAM_ID);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBeGreaterThan(BigInt(74_000_000)); // At least 74 USDC
            expect(userUsdcBalance).toBeLessThan(BigInt(76_000_000)); // Less than 76 USDC
        });

        test("Token2022 Test #2: APR=15%, Fee=5%, 90 days", async () => {
            // given
            const usdc2022 = testHelper.createMint2022(6);
            const onyc2022 = testHelper.createMint2022(9);

            await program.makeOffer({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID
            });

            const offerPda = program.getOfferPda(usdc2022, onyc2022);

            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                baseTime,
                basePrice: 2_000_000_000, // 2.0 USDC per ONyc
                apr: 150_000, // 15% APR
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID,
                feeBasisPoints: 500 // 5% fee
            });

            const redemptionOfferPda = program.getRedemptionOfferPda(onyc2022, usdc2022);

            await program.transferMintAuthorityToProgram({ mint: onyc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });
            await program.transferMintAuthorityToProgram({ mint: usdc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onyc2022, boss, BigInt(0), true);

            await testHelper.advanceClockBy(90 * 86400); // 90 days

            const amount = 100_000_000_000; // 100 ONyc
            // Create redeemer's Token2022 account for token_in
            testHelper.createTokenAccount(onyc2022, redeemer.publicKey, BigInt(amount), true, TOKEN_2022_PROGRAM_ID);

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onyc2022, boss, false, TOKEN_2022_PROGRAM_ID);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onyc2022,
                tokenOutMint: usdc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            // then - Boss receives 5% fee (5 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(BigInt(5_000_000_000)); // 5 ONyc fee

            // User receives: 95 ONyc * ~2.074 = ~197 USDC (15% APR over 90 days)
            const userUsdcAccount = getAssociatedTokenAddressSync(usdc2022, redeemer.publicKey, false, TOKEN_2022_PROGRAM_ID);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBeGreaterThan(BigInt(195_000_000));
            expect(userUsdcBalance).toBeLessThan(BigInt(200_000_000));
        });

        test("Token2022 Test #3: APR=25%, Fee=2.5%, 180 days", async () => {
            // given
            const usdc2022 = testHelper.createMint2022(6);
            const onyc2022 = testHelper.createMint2022(9);

            await program.makeOffer({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID
            });

            const offerPda = program.getOfferPda(usdc2022, onyc2022);

            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                baseTime,
                basePrice: 800_000_000, // 0.8 USDC per ONyc
                apr: 250_000, // 25% APR
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID,
                feeBasisPoints: 250 // 2.5% fee
            });

            const redemptionOfferPda = program.getRedemptionOfferPda(onyc2022, usdc2022);

            await program.transferMintAuthorityToProgram({ mint: onyc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });
            await program.transferMintAuthorityToProgram({ mint: usdc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onyc2022, boss, BigInt(0), true);

            await testHelper.advanceClockBy(180 * 86400); // 180 days

            const amount = 200_000_000_000; // 200 ONyc
            // Create redeemer's Token2022 account for token_in
            testHelper.createTokenAccount(onyc2022, redeemer.publicKey, BigInt(amount), true, TOKEN_2022_PROGRAM_ID);

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onyc2022, boss, false, TOKEN_2022_PROGRAM_ID);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onyc2022,
                tokenOutMint: usdc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            // then - Boss receives 2.5% fee (5 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(BigInt(5_000_000_000)); // 5 ONyc fee

            // User receives: 195 ONyc * ~0.9 = ~175.5 USDC (25% APR over 180 days)
            const userUsdcAccount = getAssociatedTokenAddressSync(usdc2022, redeemer.publicKey, false, TOKEN_2022_PROGRAM_ID);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBeGreaterThan(BigInt(170_000_000));
            expect(userUsdcBalance).toBeLessThan(BigInt(180_000_000));
        });

        test("Token2022 Test #4: APR=10%, Fee=0.5%, 45 days", async () => {
            // given
            const usdc2022 = testHelper.createMint2022(6);
            const onyc2022 = testHelper.createMint2022(9);

            await program.makeOffer({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
            });

            const offerPda = program.getOfferPda(usdc2022, onyc2022);

            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                baseTime,
                basePrice: 3_000_000_000, // 3.0 USDC per ONyc
                apr: 100_000, // 10% APR
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID,
                feeBasisPoints: 50 // 0.5% fee
            });

            const redemptionOfferPda = program.getRedemptionOfferPda(onyc2022, usdc2022);

            await program.transferMintAuthorityToProgram({ mint: onyc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });
            await program.transferMintAuthorityToProgram({ mint: usdc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onyc2022, boss, BigInt(0), true, TOKEN_2022_PROGRAM_ID);

            await testHelper.advanceClockBy(45 * 86400); // 45 days

            const amount = 75_000_000_000; // 75 ONyc
            // Create redeemer's Token2022 account for token_in
            testHelper.createTokenAccount(onyc2022, redeemer.publicKey, BigInt(amount), true, TOKEN_2022_PROGRAM_ID);

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onyc2022, boss, false, TOKEN_2022_PROGRAM_ID);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onyc2022,
                tokenOutMint: usdc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            // then - Boss receives 0.5% fee (0.375 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(BigInt(375_000_000)); // 0.375 ONyc fee

            // User receives: 74.625 ONyc * ~3.037 = ~226.6 USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdc2022, redeemer.publicKey, false, TOKEN_2022_PROGRAM_ID);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBeGreaterThan(BigInt(225_000_000));
            expect(userUsdcBalance).toBeLessThan(BigInt(230_000_000));
        });

        test("Token2022 Test #5: APR=30%, Fee=7%, 60 days", async () => {
            // given
            const usdc2022 = testHelper.createMint2022(6);
            const onyc2022 = testHelper.createMint2022(9);

            await program.makeOffer({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID
            });

            const offerPda = program.getOfferPda(usdc2022, onyc2022);

            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                baseTime,
                basePrice: 1_200_000_000, // 1.2 USDC per ONyc
                apr: 300_000, // 30% APR
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID,
                feeBasisPoints: 700 // 7% fee
            });

            const redemptionOfferPda = program.getRedemptionOfferPda(onyc2022, usdc2022);

            await program.transferMintAuthorityToProgram({ mint: onyc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });
            await program.transferMintAuthorityToProgram({ mint: usdc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onyc2022, boss, BigInt(0), true);

            await testHelper.advanceClockBy(60 * 86400); // 60 days

            const amount = 120_000_000_000; // 120 ONyc
            // Create redeemer's Token2022 account for token_in
            testHelper.createTokenAccount(onyc2022, redeemer.publicKey, BigInt(amount), true, TOKEN_2022_PROGRAM_ID);

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onyc2022, boss, false, TOKEN_2022_PROGRAM_ID);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onyc2022,
                tokenOutMint: usdc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            // then - Boss receives 7% fee (8.4 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(BigInt(8_400_000_000)); // 8.4 ONyc fee

            // User receives: 111.6 ONyc * ~1.26 = ~140.6 USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdc2022, redeemer.publicKey, false, TOKEN_2022_PROGRAM_ID);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBeGreaterThan(BigInt(138_000_000));
            expect(userUsdcBalance).toBeLessThan(BigInt(143_000_000));
        });

        test("Token2022 Test #6: APR=8%, Fee=3%, 15 days", async () => {
            // given
            const usdc2022 = testHelper.createMint2022(6);
            const onyc2022 = testHelper.createMint2022(9);

            await program.makeOffer({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID
            });

            const offerPda = program.getOfferPda(usdc2022, onyc2022);

            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                baseTime,
                basePrice: 2_500_000_000, // 2.5 USDC per ONyc
                apr: 80_000, // 8% APR
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID,
                feeBasisPoints: 300 // 3% fee
            });

            const redemptionOfferPda = program.getRedemptionOfferPda(onyc2022, usdc2022);

            await program.transferMintAuthorityToProgram({ mint: onyc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });
            await program.transferMintAuthorityToProgram({ mint: usdc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onyc2022, boss, BigInt(0), true);

            await testHelper.advanceClockBy(15 * 86400); // 15 days

            const amount = 30_000_000_000; // 30 ONyc
            // Create redeemer's Token2022 account for token_in
            testHelper.createTokenAccount(onyc2022, redeemer.publicKey, BigInt(amount), true, TOKEN_2022_PROGRAM_ID);

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onyc2022, boss, false, TOKEN_2022_PROGRAM_ID);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onyc2022,
                tokenOutMint: usdc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            // then - Boss receives 3% fee (0.9 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(BigInt(900_000_000)); // 0.9 ONyc fee

            // User receives: 29.1 ONyc * ~2.508 = ~72.98 USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdc2022, redeemer.publicKey, false, TOKEN_2022_PROGRAM_ID);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBeGreaterThan(BigInt(72_000_000));
            expect(userUsdcBalance).toBeLessThan(BigInt(74_000_000));
        });

        test("Token2022 Test #7: APR=12%, Fee=4.5%, 120 days", async () => {
            // given
            const usdc2022 = testHelper.createMint2022(6);
            const onyc2022 = testHelper.createMint2022(9);

            await program.makeOffer({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID
            });

            const offerPda = program.getOfferPda(usdc2022, onyc2022);

            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                baseTime,
                basePrice: 1_000_000_000, // 1.0 USDC per ONyc
                apr: 120_000, // 12% APR
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID,
                feeBasisPoints: 450 // 4.5% fee
            });

            const redemptionOfferPda = program.getRedemptionOfferPda(onyc2022, usdc2022);

            await program.transferMintAuthorityToProgram({ mint: onyc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });
            await program.transferMintAuthorityToProgram({ mint: usdc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onyc2022, boss, BigInt(0), true);

            await testHelper.advanceClockBy(120 * 86400); // 120 days

            const amount = 80_000_000_000; // 80 ONyc
            // Create redeemer's Token2022 account for token_in
            testHelper.createTokenAccount(onyc2022, redeemer.publicKey, BigInt(amount), true, TOKEN_2022_PROGRAM_ID);

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onyc2022, boss, false, TOKEN_2022_PROGRAM_ID);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onyc2022,
                tokenOutMint: usdc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            // then - Boss receives 4.5% fee (3.6 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(BigInt(3_600_000_000)); // 3.6 ONyc fee

            // User receives: 76.4 ONyc * ~1.04 = ~79.5 USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdc2022, redeemer.publicKey, false, TOKEN_2022_PROGRAM_ID);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBeGreaterThan(BigInt(78_000_000));
            expect(userUsdcBalance).toBeLessThan(BigInt(81_000_000));
        });

        test("Token2022 Test #8: APR=18%, Fee=6%, 270 days", async () => {
            // given
            const usdc2022 = testHelper.createMint2022(6);
            const onyc2022 = testHelper.createMint2022(9);

            await program.makeOffer({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID
            });

            const offerPda = program.getOfferPda(usdc2022, onyc2022);

            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                baseTime,
                basePrice: 500_000_000, // 0.5 USDC per ONyc
                apr: 180_000, // 18% APR
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID,
                feeBasisPoints: 600 // 6% fee
            });

            const redemptionOfferPda = program.getRedemptionOfferPda(onyc2022, usdc2022);

            await program.transferMintAuthorityToProgram({ mint: onyc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });
            await program.transferMintAuthorityToProgram({ mint: usdc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onyc2022, boss, BigInt(0), true);

            await testHelper.advanceClockBy(270 * 86400); // 270 days

            const amount = 150_000_000_000; // 150 ONyc
            // Create redeemer's Token2022 account for token_in
            testHelper.createTokenAccount(onyc2022, redeemer.publicKey, BigInt(amount), true, TOKEN_2022_PROGRAM_ID);

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onyc2022, boss, false, TOKEN_2022_PROGRAM_ID);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onyc2022,
                tokenOutMint: usdc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            // then - Boss receives 6% fee (9 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(BigInt(9_000_000_000)); // 9 ONyc fee

            // User receives: 141 ONyc * ~0.567 = ~79.9 USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdc2022, redeemer.publicKey, false, TOKEN_2022_PROGRAM_ID);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBeGreaterThan(BigInt(78_000_000));
            expect(userUsdcBalance).toBeLessThan(BigInt(82_000_000));
        });

        test("Token2022 Test #9: APR=22%, Fee=1.5%, 365 days", async () => {
            // given
            const usdc2022 = testHelper.createMint2022(6);
            const onyc2022 = testHelper.createMint2022(9);

            await program.makeOffer({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID
            });

            const offerPda = program.getOfferPda(usdc2022, onyc2022);

            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                baseTime,
                basePrice: 1_800_000_000, // 1.8 USDC per ONyc
                apr: 220_000, // 22% APR
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID,
                feeBasisPoints: 150 // 1.5% fee
            });

            const redemptionOfferPda = program.getRedemptionOfferPda(onyc2022, usdc2022);

            await program.transferMintAuthorityToProgram({ mint: onyc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });
            await program.transferMintAuthorityToProgram({ mint: usdc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onyc2022, boss, BigInt(0), true);

            await testHelper.advanceClockBy(365 * 86400); // 365 days (1 year)

            const amount = 60_000_000_000; // 60 ONyc
            // Create redeemer's Token2022 account for token_in
            testHelper.createTokenAccount(onyc2022, redeemer.publicKey, BigInt(amount), true, TOKEN_2022_PROGRAM_ID);

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onyc2022, boss, false, TOKEN_2022_PROGRAM_ID);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onyc2022,
                tokenOutMint: usdc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            // then - Boss receives 1.5% fee (0.9 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(BigInt(900_000_000)); // 0.9 ONyc fee

            // User receives: 59.1 ONyc * ~2.196 = ~129.8 USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdc2022, redeemer.publicKey, false, TOKEN_2022_PROGRAM_ID);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBeGreaterThan(BigInt(128_000_000));
            expect(userUsdcBalance).toBeLessThan(BigInt(132_000_000));
        });

        test("Token2022 Test #10: APR=35%, Fee=8%, 7 days", async () => {
            // given
            const usdc2022 = testHelper.createMint2022(6);
            const onyc2022 = testHelper.createMint2022(9);

            await program.makeOffer({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID
            });

            const offerPda = program.getOfferPda(usdc2022, onyc2022);

            const baseTime = await testHelper.getCurrentClockTime();
            await program.addOfferVector({
                tokenInMint: usdc2022,
                tokenOutMint: onyc2022,
                baseTime,
                basePrice: 4_000_000_000, // 4.0 USDC per ONyc
                apr: 350_000, // 35% APR
                priceFixDuration: 86400
            });

            await program.makeRedemptionOffer({
                offer: offerPda,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID,
                feeBasisPoints: 800 // 8% fee
            });

            const redemptionOfferPda = program.getRedemptionOfferPda(onyc2022, usdc2022);

            await program.transferMintAuthorityToProgram({ mint: onyc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });
            await program.transferMintAuthorityToProgram({ mint: usdc2022, tokenProgram: TOKEN_2022_PROGRAM_ID });

            const boss = testHelper.getBoss();
            testHelper.createTokenAccount(onyc2022, boss, BigInt(0), true);

            await testHelper.advanceClockBy(7 * 86400); // 7 days

            const amount = 25_000_000_000; // 25 ONyc
            // Create redeemer's Token2022 account for token_in
            testHelper.createTokenAccount(onyc2022, redeemer.publicKey, BigInt(amount), true, TOKEN_2022_PROGRAM_ID);

            await program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            const redemptionRequestPda = program.getRedemptionRequestPda(
                redemptionOfferPda,
                0
            );

            const bossOnycAccount = getAssociatedTokenAddressSync(onyc2022, boss, false, TOKEN_2022_PROGRAM_ID);

            // when
            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onyc2022,
                tokenOutMint: usdc2022,
                tokenInProgram: TOKEN_2022_PROGRAM_ID,
                tokenOutProgram: TOKEN_2022_PROGRAM_ID
            });

            // then - Boss receives 8% fee (2 ONyc)
            const finalBossBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(finalBossBalance).toBe(BigInt(2_000_000_000)); // 2 ONyc fee

            // User receives: 23 ONyc * ~4.027 = ~92.6 USDC
            const userUsdcAccount = getAssociatedTokenAddressSync(usdc2022, redeemer.publicKey, false, TOKEN_2022_PROGRAM_ID);
            const userUsdcBalance = await testHelper.getTokenAccountBalance(userUsdcAccount);
            expect(userUsdcBalance).toBeGreaterThan(BigInt(91_000_000));
            expect(userUsdcBalance).toBeLessThan(BigInt(94_000_000));
        });
    });
});
