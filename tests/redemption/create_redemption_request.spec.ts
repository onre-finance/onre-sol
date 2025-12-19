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
        const expiresAt = Math.floor(Date.now() / 1000) + 3600; // 1 hour from now

        // when
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            amount: REDEMPTION_AMOUNT,
            expiresAt
        });

        // then - counter starts at 0
        const redemptionRequest = await program.getRedemptionRequest(
            redemptionOfferPda,
            0
        );

        expect(redemptionRequest.offer.toString()).toBe(redemptionOfferPda.toString());
        expect(redemptionRequest.redeemer.toString()).toBe(redeemer.publicKey.toString());
        expect(redemptionRequest.amount.toString()).toBe(REDEMPTION_AMOUNT.toString());
    });

    test("Should increment counter after each request", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // Check initial counter
        const initialOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(initialOffer.counter.toString()).toBe("0");

        // First request
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            amount: REDEMPTION_AMOUNT,
            expiresAt
        });

        // Check counter after first request
        const offerAfterFirst = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(offerAfterFirst.counter.toString()).toBe("1");

        // Second request (different redeemer to avoid same tx issue)
        const redeemer2 = testHelper.createUserAccount();
        testHelper.createTokenAccount(onycMint, redeemer2.publicKey, BigInt(10_000_000_000));

        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer: redeemer2,
            amount: REDEMPTION_AMOUNT,
            expiresAt
        });

        // then
        const offerAfterSecond = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(offerAfterSecond.counter.toString()).toBe("2");
    });

    test("Should update requested_redemptions in RedemptionOffer", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // when
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            amount: REDEMPTION_AMOUNT,
            expiresAt
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
            amount: REDEMPTION_AMOUNT,
            expiresAt
        });

        // Second redeemer makes a request
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer: redeemer2,
            amount: REDEMPTION_AMOUNT * 2,
            expiresAt
        });

        // then
        const redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.requestedRedemptions.toString()).toBe(
            (REDEMPTION_AMOUNT * 3).toString()
        );
    });

    test("Should reject when expires_at is in the past", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) - 3600; // 1 hour ago

        // when/then
        await expect(
            program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT,
                expiresAt
            })
        ).rejects.toThrow();
    });

    test("Should create unique PDAs for different counters", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;
        const redeemer2 = testHelper.createUserAccount();
        testHelper.createTokenAccount(onycMint, redeemer2.publicKey, BigInt(10_000_000_000));

        // when - Create multiple requests (each gets unique counter)
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            amount: REDEMPTION_AMOUNT,
            expiresAt
        });

        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer: redeemer2,
            amount: REDEMPTION_AMOUNT * 2,
            expiresAt
        });

        // then - Both requests should exist with different amounts
        const request1 = await program.getRedemptionRequest(
            redemptionOfferPda,
            0
        );
        const request2 = await program.getRedemptionRequest(
            redemptionOfferPda,
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
            amount: REDEMPTION_AMOUNT,
            expiresAt
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
            amount: REDEMPTION_AMOUNT,
            expiresAt
        });

        // then
        const updatedRedeemerTokenAccount = await testHelper.getTokenAccount(redeemerTokenAccountAddress);
        const vaultTokenAccount = await testHelper.getTokenAccount(vaultTokenAccountAddress);

        expect(updatedRedeemerTokenAccount.amount).toBe(initialRedeemerBalance - BigInt(REDEMPTION_AMOUNT));
        expect(vaultTokenAccount.amount).toBe(initialVaultBalance + BigInt(REDEMPTION_AMOUNT));
    });

    test("Should reject when kill switch is activated", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // Activate kill switch
        await program.setKillSwitch({ enable: true });

        // when/then
        await expect(
            program.createRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redeemer,
                amount: REDEMPTION_AMOUNT,
                expiresAt
            })
        ).rejects.toThrow("Redemption system is paused: kill switch activated");
    });

    test("Should succeed after kill switch is deactivated", async () => {
        // given
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // Activate then deactivate kill switch
        await program.setKillSwitch({ enable: true });
        await program.setKillSwitch({ enable: false });

        // when
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            amount: REDEMPTION_AMOUNT,
            expiresAt
        });

        // then
        const redemptionRequest = await program.getRedemptionRequest(
            redemptionOfferPda,
            0
        );

        expect(redemptionRequest.amount.toString()).toBe(REDEMPTION_AMOUNT.toString());
    });

    test("Anyone can create a redemption request without admin approval", async () => {
        // given - a random user with tokens
        const randomUser = testHelper.createUserAccount();
        testHelper.createTokenAccount(onycMint, randomUser.publicKey, BigInt(10_000_000_000));
        const expiresAt = Math.floor(Date.now() / 1000) + 3600;

        // when - create request without any admin involvement
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer: randomUser,
            amount: REDEMPTION_AMOUNT,
            expiresAt
        });

        // then - request should be created
        const redemptionRequest = await program.getRedemptionRequest(
            redemptionOfferPda,
            0
        );

        expect(redemptionRequest.redeemer.toString()).toBe(randomUser.publicKey.toString());
        expect(redemptionRequest.amount.toString()).toBe(REDEMPTION_AMOUNT.toString());
    });
});
