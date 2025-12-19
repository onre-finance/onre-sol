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
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redeemer,
            redemptionAdmin: redemptionAdmin.publicKey
        });

        // then - account should be closed
        await expect(
            program.getRedemptionRequest(redemptionOfferPda, 0)
        ).rejects.toThrow();
    });

    test("Should cancel redemption request as redemption_admin", async () => {
        // given
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
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redemptionAdmin,
            redemptionAdmin: redemptionAdmin.publicKey
        });

        // then - account should be closed
        await expect(
            program.getRedemptionRequest(redemptionOfferPda, 0)
        ).rejects.toThrow();
    });

    test("Should cancel redemption request as boss", async () => {
        // given
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
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: boss,
            redemptionAdmin: redemptionAdmin.publicKey
        });

        // then - account should be closed
        await expect(
            program.getRedemptionRequest(redemptionOfferPda, 0)
        ).rejects.toThrow();
    });

    test("Should subtract amount from requested_redemptions", async () => {
        // given
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            amount: REDEMPTION_AMOUNT
        });

        // Verify requested_redemptions was incremented
        let redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe(REDEMPTION_AMOUNT.toString());

        const redemptionRequestPda = program.getRedemptionRequestPda(
            redemptionOfferPda,
            0
        );

        // when
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redeemer,
            redemptionAdmin: redemptionAdmin.publicKey
        });

        // then
        redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe("0");
    });

    test("Should correctly handle multiple cancellations", async () => {
        // given
        const redeemer2 = testHelper.createUserAccount();
        testHelper.createTokenAccount(onycMint, redeemer2.publicKey, BigInt(10_000_000_000)); // 10 ONyc

        // Create two requests (each gets a unique counter)
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

        // Verify total requested_redemptions
        let redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe(
            (REDEMPTION_AMOUNT * 3).toString()
        );

        // when - Cancel first request (counter 0)
        const redemptionRequestPda1 = program.getRedemptionRequestPda(
            redemptionOfferPda,
            0
        );

        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda1,
            signer: redeemer,
            redemptionAdmin: redemptionAdmin.publicKey
        });

        // then - Should be decremented by REDEMPTION_AMOUNT
        redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe(
            (REDEMPTION_AMOUNT * 2).toString()
        );

        // when - Cancel second request (counter 1)
        const redemptionRequestPda2 = program.getRedemptionRequestPda(
            redemptionOfferPda,
            1
        );

        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda2,
            signer: redeemer2,
            redemptionAdmin: redemptionAdmin.publicKey
        });

        // then - Should be back to 0
        redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe("0");
    });

    test("Should close the redemption request account and return rent to redemption_admin", async () => {
        // given
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
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redeemer,
            redemptionAdmin: redemptionAdmin.publicKey
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

    test("Should reject when signer is unauthorized", async () => {
        // given
        const unauthorizedUser = testHelper.createUserAccount();

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
            program.cancelRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                signer: unauthorizedUser,
                redemptionAdmin: redemptionAdmin.publicKey
            })
        ).rejects.toThrow();
    });

    test("Should allow cancelling one request while others remain active", async () => {
        // given
        const redeemer2 = testHelper.createUserAccount();
        testHelper.createTokenAccount(onycMint, redeemer2.publicKey, BigInt(10_000_000_000));

        // Create two requests
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

        const redemptionRequestPda1 = program.getRedemptionRequestPda(
            redemptionOfferPda,
            0
        );

        // when - Cancel only the first request
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda1,
            signer: redeemer,
            redemptionAdmin: redemptionAdmin.publicKey
        });

        // then - First request should be closed
        await expect(
            program.getRedemptionRequest(redemptionOfferPda, 0)
        ).rejects.toThrow();

        // Second request should still exist
        const request2 = await program.getRedemptionRequest(
            redemptionOfferPda,
            1
        );
        expect(request2.amount.toString()).toBe((REDEMPTION_AMOUNT * 2).toString());

        // Verify requested_redemptions only decremented by first amount
        const redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe(
            (REDEMPTION_AMOUNT * 2).toString()
        );
    });

    test("Should return locked tokens to redeemer", async () => {
        // given
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
            amount: REDEMPTION_AMOUNT
        });

        // Verify tokens were locked
        const afterLockRedeemerTokenAccount = await testHelper.getTokenAccount(redeemerTokenAccountAddress);
        expect(afterLockRedeemerTokenAccount.amount).toBe(initialRedeemerBalance - BigInt(REDEMPTION_AMOUNT));

        const afterLockVaultTokenAccount = await testHelper.getTokenAccount(vaultTokenAccountAddress);
        expect(afterLockVaultTokenAccount.amount).toBe(initialVaultBalance + BigInt(REDEMPTION_AMOUNT));

        const redemptionRequestPda = program.getRedemptionRequestPda(
            redemptionOfferPda,
            0
        );

        // when - Cancel the request
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redeemer,
            redemptionAdmin: redemptionAdmin.publicKey
        });

        // then - Tokens should be returned
        const finalRedeemerTokenAccount = await testHelper.getTokenAccount(redeemerTokenAccountAddress);
        expect(finalRedeemerTokenAccount.amount).toBe(initialRedeemerBalance);

        // Vault should be back to initial balance
        const finalVaultTokenAccount = await testHelper.getTokenAccount(vaultTokenAccountAddress);
        expect(finalVaultTokenAccount.amount).toBe(initialVaultBalance);
    });

    test("Should reject when kill switch is activated", async () => {
        // given - Create redemption request first
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
            program.cancelRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                signer: redeemer,
                redemptionAdmin: redemptionAdmin.publicKey
            })
        ).rejects.toThrow("Operation not allowed: program is in kill switch state");
    });

    test("Should succeed after kill switch is deactivated", async () => {
        // given - Create redemption request first
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            amount: REDEMPTION_AMOUNT
        });

        const redemptionRequestPda = program.getRedemptionRequestPda(
            redemptionOfferPda,
            0
        );

        // Activate then deactivate kill switch
        await program.setKillSwitch({ enable: true });
        await program.setKillSwitch({ enable: false });

        // when
        await program.cancelRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            signer: redeemer,
            redemptionAdmin: redemptionAdmin.publicKey
        });

        // then - account should be closed
        await expect(
            program.getRedemptionRequest(redemptionOfferPda, 0)
        ).rejects.toThrow();
    });
});
