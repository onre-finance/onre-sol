import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

describe("Create redemption request", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let usdcMint: PublicKey;
    let onycMint: PublicKey;
    let offerPda: PublicKey;
    let redemptionOfferPda: PublicKey;
    let redemptionAdmin: Keypair;
    let redeemer: Keypair;

    const REDEMPTION_AMOUNT = 1_000_000_000; // 1 ONyc

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints
        usdcMint = testHelper.createMint(6);
        onycMint = testHelper.createMint(9);

        // Initialize program
        await program.initialize({ onycMint });

        // Set redemption admin
        redemptionAdmin = testHelper.createUserAccount();
        await program.setRedemptionAdmin({ redemptionAdmin: redemptionAdmin.publicKey });

        // Create user accounts
        redeemer = testHelper.createUserAccount();

        // Create token account for redeemer with ONyc tokens
        testHelper.createTokenAccount(onycMint, redeemer.publicKey, BigInt(10_000_000_000)); // 10 ONyc

        // Create token account for boss with ONyc tokens for vault deposit
        const bossPublicKey = testHelper.context.payer.publicKey;
        testHelper.createTokenAccount(onycMint, bossPublicKey, BigInt(100_000_000_000)); // 100 ONyc

        // Create the base offer (USDC -> ONyc)
        await program.makeOffer({
            tokenInMint: usdcMint,
            tokenOutMint: onycMint
        });

        offerPda = program.getOfferPda(usdcMint, onycMint);

        // Create redemption offer (ONyc -> USDC)
        await program.makeRedemptionOffer({
            offer: offerPda
        });

        redemptionOfferPda = program.getRedemptionOfferPda(onycMint, usdcMint);

        // Deposit into redemption vault to create the vault token account
        await program.redemptionVaultDeposit({
            amount: 1_000_000_000, // 1 ONyc (enough for initial setup)
            tokenMint: onycMint
        });
    });

    test("Create redemption request should succeed with valid params", async () => {
        // given
        const nonce = 0;
        const expiresAt = Math.floor(Date.now() / 1000) + 3600; // 1 hour from now

        // when
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt,
            nonce
        });

        // then
        const redemptionRequest = await program.getRedemptionRequest(
            redemptionOfferPda,
            redeemer.publicKey,
            nonce
        );

        expect(redemptionRequest.offer.toString()).toBe(redemptionOfferPda.toString());
        expect(redemptionRequest.redeemer.toString()).toBe(redeemer.publicKey.toString());
        expect(redemptionRequest.amount.toString()).toBe(REDEMPTION_AMOUNT.toString());
        expect(redemptionRequest.status).toBe(0); // Pending
    });

    test("Should initialize UserNonceAccount on first request", async () => {
        // given
        const nonce = 0;
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // when
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt,
            nonce
        });

        // then
        const userNonceAccount = await program.getUserNonceAccount(redeemer.publicKey);
        expect(userNonceAccount.nonce.toString()).toBe("1"); // Should be incremented to 1
    });

    test("Should increment nonce after each request", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // First request
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt,
            nonce: 0
        });

        // Second request
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt: expiresAt + 1000,
            nonce: 1
        });

        // then
        const userNonceAccount = await program.getUserNonceAccount(redeemer.publicKey);
        expect(userNonceAccount.nonce.toString()).toBe("2");
    });

    test("Should update requested_redemptions in RedemptionOffer", async () => {
        // given
        const nonce = 0;
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // when
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt,
            nonce
        });

        // then
        const redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe(REDEMPTION_AMOUNT.toString());
    });

    test("Should accumulate requested_redemptions from multiple requests", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;
        const redeemer2 = testHelper.createUserAccount();
        testHelper.createTokenAccount(onycMint, redeemer2.publicKey, BigInt(10_000_000_000)); // 10 ONyc

        // when - First redeemer makes a request
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt,
            nonce: 0
        });

        // Second redeemer makes a request
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer: redeemer2,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT * 2,
            expiresAt,
            nonce: 0
        });

        // then
        const redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe(
            (REDEMPTION_AMOUNT * 3).toString()
        );
    });

    test("Should reject when nonce doesn't match", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // when/then - Try with wrong nonce
        await expect(
            program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce: 5 // Wrong nonce, should be 0
            })
        ).rejects.toThrow();
    });

    test("Should reject when redemption_admin is not authorized", async () => {
        // given
        const unauthorizedAdmin = testHelper.createUserAccount();
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // when/then
        await expect(
            program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin: unauthorizedAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce: 0
            })
        ).rejects.toThrow();
    });

    test("Should reject when expires_at is in the past", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) - 3600; // 1 hour ago

        // when/then
        await expect(
            program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt,
                nonce: 0
            })
        ).rejects.toThrow();
    });

    test("Should prevent replay attacks with same nonce", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // First request succeeds
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt,
            nonce: 0
        });

        // when/then - Try to reuse same nonce (nonce is now 1, not 0)
        await expect(
            program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                redemptionAdmin,
                amount: REDEMPTION_AMOUNT,
                expiresAt: expiresAt + 1000,
                nonce: 0
            })
        ).rejects.toThrow();
    });

    test("Should allow multiple users to create requests with same nonce value", async () => {
        // given
        const redeemer2 = testHelper.createUserAccount();
        testHelper.createTokenAccount(onycMint, redeemer2.publicKey, BigInt(10_000_000_000)); // 10 ONyc
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;
        const nonce = 0;

        // when - Both users create requests with nonce 0 (their first request)
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt,
            nonce
        });

        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer: redeemer2,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt,
            nonce
        });

        // then - Both requests should exist
        const request1 = await program.getRedemptionRequest(
            redemptionOfferPda,
            redeemer.publicKey,
            nonce
        );
        const request2 = await program.getRedemptionRequest(
            redemptionOfferPda,
            redeemer2.publicKey,
            nonce
        );

        expect(request1.redeemer.toString()).toBe(redeemer.publicKey.toString());
        expect(request2.redeemer.toString()).toBe(redeemer2.publicKey.toString());
    });

    test("Should create unique PDAs for different nonces", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // when - Create multiple requests with different nonces
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
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT * 2,
            expiresAt: expiresAt + 1000,
            nonce: 1
        });

        // then - Both requests should exist with different amounts
        const request1 = await program.getRedemptionRequest(
            redemptionOfferPda,
            redeemer.publicKey,
            0
        );
        const request2 = await program.getRedemptionRequest(
            redemptionOfferPda,
            redeemer.publicKey,
            1
        );

        expect(request1.amount.toString()).toBe(REDEMPTION_AMOUNT.toString());
        expect(request2.amount.toString()).toBe((REDEMPTION_AMOUNT * 2).toString());
    });

    test("Redeemer pays for account creation", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;
        const initialBalance = await testHelper.context.banksClient.getBalance(
            redeemer.publicKey
        );

        // when
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt,
            nonce: 0
        });

        // then
        const finalBalance = await testHelper.context.banksClient.getBalance(
            redeemer.publicKey
        );

        // Balance should decrease (account rent + transaction fees)
        expect(finalBalance).toBeLessThan(initialBalance);
    });

    test("Should lock tokens in redemption vault", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;
        const redeemerTokenAccountAddress = getAssociatedTokenAddressSync(onycMint, redeemer.publicKey);
        const redeemerTokenAccount = await testHelper.getTokenAccount(redeemerTokenAccountAddress);
        const initialRedeemerBalance = redeemerTokenAccount.amount;

        const vaultTokenAccountAddress = getAssociatedTokenAddressSync(
            onycMint,
            program.pdas.redemptionVaultAuthorityPda,
            true
        );
        const initialVaultTokenAccount = await testHelper.getTokenAccount(vaultTokenAccountAddress);
        const initialVaultBalance = initialVaultTokenAccount.amount;

        // when
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt,
            nonce: 0
        });

        // then
        const updatedRedeemerTokenAccount = await testHelper.getTokenAccount(redeemerTokenAccountAddress);
        const vaultTokenAccount = await testHelper.getTokenAccount(vaultTokenAccountAddress);

        expect(updatedRedeemerTokenAccount.amount).toBe(initialRedeemerBalance - BigInt(REDEMPTION_AMOUNT));
        expect(vaultTokenAccount.amount).toBe(initialVaultBalance + BigInt(REDEMPTION_AMOUNT));
    });
});
