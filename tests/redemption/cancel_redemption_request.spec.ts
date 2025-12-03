import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

describe("Cancel redemption request", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let usdcMint: PublicKey;
    let onycMint: PublicKey;
    let offerPda: PublicKey;
    let redemptionOfferPda: PublicKey;
    let redemptionAdmin: Keypair;
    let redeemer: Keypair;
    let boss: Keypair;

    const REDEMPTION_AMOUNT = 1_000_000_000; // 1 ONyc

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Store boss keypair
        boss = testHelper.context.payer;

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
        testHelper.createTokenAccount(onycMint, boss.publicKey, BigInt(100_000_000_000)); // 100 ONyc

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

    test("Should cancel redemption request as redeemer", async () => {
        // given
        const nonce = 0;
        const expiresAt = Math.floor(Date.now() / 1000) + 3600; // 1 hour from now

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
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redeemer
        });

        // then
        const redemptionRequest = await program.getRedemptionRequest(
            redemptionOfferPda,
            redeemer.publicKey,
            nonce
        );

        expect(redemptionRequest.status).toBe(2); // Cancelled
    });

    test("Should cancel redemption request as redemption_admin", async () => {
        // given
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
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redemptionAdmin
        });

        // then
        const redemptionRequest = await program.getRedemptionRequest(
            redemptionOfferPda,
            redeemer.publicKey,
            nonce
        );

        expect(redemptionRequest.status).toBe(2); // Cancelled
    });

    test("Should cancel redemption request as boss", async () => {
        // given
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
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: boss
        });

        // then
        const redemptionRequest = await program.getRedemptionRequest(
            redemptionOfferPda,
            redeemer.publicKey,
            nonce
        );

        expect(redemptionRequest.status).toBe(2); // Cancelled
    });

    test("Should subtract amount from requested_redemptions", async () => {
        // given
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

        // Verify requested_redemptions was incremented
        let redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe(REDEMPTION_AMOUNT.toString());

        const redemptionRequestPda = program.getRedemptionRequestPda(
            redemptionOfferPda,
            redeemer.publicKey,
            nonce
        );

        // when
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redeemer
        });

        // then
        redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe("0");
    });

    test("Should correctly handle multiple cancellations", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;
        const redeemer2 = testHelper.createUserAccount();
        testHelper.createTokenAccount(onycMint, redeemer2.publicKey, BigInt(10_000_000_000)); // 10 ONyc

        // Create two requests
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

        // Verify total requested_redemptions
        let redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe(
            (REDEMPTION_AMOUNT * 3).toString()
        );

        // when - Cancel first request
        const redemptionRequestPda1 = program.getRedemptionRequestPda(
            redemptionOfferPda,
            redeemer.publicKey,
            0
        );

        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda1,
            signer: redeemer
        });

        // then - Should be decremented by REDEMPTION_AMOUNT
        redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe(
            (REDEMPTION_AMOUNT * 2).toString()
        );

        // when - Cancel second request
        const redemptionRequestPda2 = program.getRedemptionRequestPda(
            redemptionOfferPda,
            redeemer2.publicKey,
            0
        );

        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda2,
            signer: redeemer2
        });

        // then - Should be back to 0
        redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe("0");
    });

    test("Should NOT close the redemption request account", async () => {
        // given
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
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redeemer
        });

        // then - Account should still exist and be fetchable
        const redemptionRequest = await program.getRedemptionRequest(
            redemptionOfferPda,
            redeemer.publicKey,
            nonce
        );

        expect(redemptionRequest).toBeDefined();
        expect(redemptionRequest.status).toBe(2); // Cancelled
        expect(redemptionRequest.amount.toString()).toBe(REDEMPTION_AMOUNT.toString());
    });

    test("Should reject when signer is unauthorized", async () => {
        // given
        const nonce = 0;
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;
        const unauthorizedUser = testHelper.createUserAccount();

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
            program.cancelRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                signer: unauthorizedUser
            })
        ).rejects.toThrow();
    });

    test("Should reject when request is already cancelled", async () => {
        // given
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

        // First cancellation succeeds
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redeemer
        });

        // when/then - Second cancellation should fail
        await expect(
            program.cancelRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                signer: redeemer
            })
        ).rejects.toThrow();
    });

    test("Should preserve all other redemption request fields", async () => {
        // given
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

        // Get original values
        const originalRequest = await program.getRedemptionRequest(
            redemptionOfferPda,
            redeemer.publicKey,
            nonce
        );

        // when
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redeemer
        });

        // then
        const cancelledRequest = await program.getRedemptionRequest(
            redemptionOfferPda,
            redeemer.publicKey,
            nonce
        );

        expect(cancelledRequest.offer.toString()).toBe(originalRequest.offer.toString());
        expect(cancelledRequest.redeemer.toString()).toBe(originalRequest.redeemer.toString());
        expect(cancelledRequest.amount.toString()).toBe(originalRequest.amount.toString());
        expect(cancelledRequest.status).toBe(2); // Only status should change
    });

    test("Should allow cancelling one request while others remain active", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // Create two requests for the same user (different nonces)
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

        const redemptionRequestPda1 = program.getRedemptionRequestPda(
            redemptionOfferPda,
            redeemer.publicKey,
            0
        );

        // when - Cancel only the first request
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda1,
            signer: redeemer
        });

        // then
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

        expect(request1.status).toBe(2); // Cancelled
        expect(request2.status).toBe(0); // Still pending

        // Verify requested_redemptions only decremented by first amount
        const redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe(
            (REDEMPTION_AMOUNT * 2).toString()
        );
    });

    test("Should return locked tokens to redeemer", async () => {
        // given
        const nonce = 0;
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;
        const redeemerTokenAccountAddress = getAssociatedTokenAddressSync(onycMint, redeemer.publicKey);

        // Get initial balance
        const initialRedeemerTokenAccount = await testHelper.getTokenAccount(redeemerTokenAccountAddress);
        const initialRedeemerBalance = initialRedeemerTokenAccount.amount;

        const vaultTokenAccountAddress = getAssociatedTokenAddressSync(
            onycMint,
            program.pdas.redemptionVaultAuthorityPda,
            true
        );
        const initialVaultTokenAccount = await testHelper.getTokenAccount(vaultTokenAccountAddress);
        const initialVaultBalance = initialVaultTokenAccount.amount;

        // Create redemption request (locks tokens)
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            redemptionAdmin,
            amount: REDEMPTION_AMOUNT,
            expiresAt,
            nonce
        });

        // Verify tokens were locked
        const afterLockRedeemerTokenAccount = await testHelper.getTokenAccount(redeemerTokenAccountAddress);
        expect(afterLockRedeemerTokenAccount.amount).toBe(initialRedeemerBalance - BigInt(REDEMPTION_AMOUNT));

        const afterLockVaultTokenAccount = await testHelper.getTokenAccount(vaultTokenAccountAddress);
        expect(afterLockVaultTokenAccount.amount).toBe(initialVaultBalance + BigInt(REDEMPTION_AMOUNT));

        const redemptionRequestPda = program.getRedemptionRequestPda(
            redemptionOfferPda,
            redeemer.publicKey,
            nonce
        );

        // when - Cancel the request
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redeemer
        });

        // then - Tokens should be returned
        const finalRedeemerTokenAccount = await testHelper.getTokenAccount(redeemerTokenAccountAddress);
        expect(finalRedeemerTokenAccount.amount).toBe(initialRedeemerBalance);

        // Vault should be back to initial balance
        const finalVaultTokenAccount = await testHelper.getTokenAccount(vaultTokenAccountAddress);
        expect(finalVaultTokenAccount.amount).toBe(initialVaultBalance);
    });
});
