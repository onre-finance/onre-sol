import { Keypair, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Fee routing in fulfill_redemption_request", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let usdcMint: PublicKey;
    let onycMint: PublicKey;
    let offerPda: PublicKey;
    let redemptionOfferPda: PublicKey;
    let redemptionAdmin: Keypair;
    let redeemer: Keypair;

    const REDEMPTION_AMOUNT = 1_000_000_000; // 1 ONyc (9 decimals)
    const FEE_BASIS_POINTS = 100; // 1%
    // fee = ceil(1_000_000_000 * 100 / 10_000) = 10_000_000
    const EXPECTED_FEE = 10_000_000;
    // token_out = net * price * 10^6 / (10^9 * 10^9) = 990_000_000 * 1e9 * 1e6 / 1e18 = 990_000
    const EXPECTED_TOKEN_OUT = 990_000; // 0.99 USDC (6 decimals)

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        usdcMint = testHelper.createMint(6);
        onycMint = testHelper.createMint(9);

        await program.initialize({ onycMint });

        redemptionAdmin = testHelper.createUserAccount();
        await program.setRedemptionAdmin({ redemptionAdmin: redemptionAdmin.publicKey });

        redeemer = testHelper.createUserAccount();

        await program.makeOffer({ tokenInMint: usdcMint, tokenOutMint: onycMint });
        offerPda = program.getOfferPda(usdcMint, onycMint);

        const currentTime = await testHelper.getCurrentClockTime();
        await program.addOfferVector({
            tokenInMint: usdcMint,
            tokenOutMint: onycMint,
            baseTime: currentTime,
            basePrice: 1e9, // 1.0
            apr: 0,
            priceFixDuration: 86400
        });

        await program.setMainOffer({ offer: offerPda });

        await program.makeRedemptionOffer({ offer: offerPda });
        redemptionOfferPda = program.getRedemptionOfferPda(onycMint, usdcMint);

        // Set fee to 1%
        await program.updateRedemptionOfferFee({
            redemptionOffer: redemptionOfferPda,
            newFeeBasisPoints: FEE_BASIS_POINTS
        });

        // Fund redeemer with ONyc
        testHelper.createTokenAccount(onycMint, redeemer.publicKey, BigInt(10_000e9), true);

        // Transfer mint authority for burn+mint path
        await program.transferMintAuthorityToProgram({ mint: onycMint });
        await program.transferMintAuthorityToProgram({ mint: usdcMint });

        // Boss token accounts (required by instruction)
        const boss = testHelper.getBoss();
        testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
        testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);
    });

    async function createAndFulfill(feeDestination?: PublicKey) {
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            amount: REDEMPTION_AMOUNT
        });

        const offer = await program.getRedemptionOffer(onycMint, usdcMint);
        const counter = offer.requestCounter.toNumber() - 1;
        const redemptionRequestPda = program.getRedemptionRequestPda(redemptionOfferPda, counter);

        await program.fulfillRedemptionRequest({
            offer: offerPda,
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            redeemer: redeemer.publicKey,
            redemptionAdmin,
            tokenInMint: onycMint,
            tokenOutMint: usdcMint,
            feeDestination
        });
    }

    test("Fees accumulate in fee vault PDA ATA when fee_destination is default", async () => {
        await createAndFulfill();

        const feeVaultPda = program.pdas.redemptionFeeVaultAuthorityPda;
        const feeVaultAta = getAssociatedTokenAddressSync(onycMint, feeVaultPda, true);
        const balance = await testHelper.getTokenAccountBalance(feeVaultAta);

        expect(Number(balance)).toBe(EXPECTED_FEE);
    });

    test("Fees go to custom wallet ATA when fee_destination is set", async () => {
        const customWallet = testHelper.createUserAccount();
        await program.setRedemptionFeeDestination({ feeDestination: customWallet.publicKey });

        await createAndFulfill(customWallet.publicKey);

        // Custom wallet ATA should have the fee
        const customAta = getAssociatedTokenAddressSync(onycMint, customWallet.publicKey, false);
        const customBalance = await testHelper.getTokenAccountBalance(customAta);
        expect(Number(customBalance)).toBe(EXPECTED_FEE);

        // Fee vault PDA ATA should be empty or non-existent (no fees routed there)
        const feeVaultPda = program.pdas.redemptionFeeVaultAuthorityPda;
        const feeVaultAta = getAssociatedTokenAddressSync(onycMint, feeVaultPda, true);
        let vaultBalance: bigint;
        try {
            vaultBalance = await testHelper.getTokenAccountBalance(feeVaultAta);
        } catch {
            vaultBalance = BigInt(0);
        }
        expect(Number(vaultBalance)).toBe(0);
    });

    test("First fulfillment fees go to vault PDA, second to new custom wallet", async () => {
        // First fulfillment with default destination
        await createAndFulfill();

        const feeVaultPda = program.pdas.redemptionFeeVaultAuthorityPda;
        const feeVaultAta = getAssociatedTokenAddressSync(onycMint, feeVaultPda, true);
        const vaultBalanceAfterFirst = await testHelper.getTokenAccountBalance(feeVaultAta);
        expect(Number(vaultBalanceAfterFirst)).toBe(EXPECTED_FEE);

        // Change destination to a new wallet
        const customWallet = testHelper.createUserAccount();
        await program.setRedemptionFeeDestination({ feeDestination: customWallet.publicKey });

        // Second fulfillment with new destination
        await createAndFulfill(customWallet.publicKey);

        // Vault PDA still has only the first fee
        const vaultBalanceAfterSecond = await testHelper.getTokenAccountBalance(feeVaultAta);
        expect(Number(vaultBalanceAfterSecond)).toBe(EXPECTED_FEE);

        // Custom wallet has the second fee
        const customAta = getAssociatedTokenAddressSync(onycMint, customWallet.publicKey, false);
        const customBalance = await testHelper.getTokenAccountBalance(customAta);
        expect(Number(customBalance)).toBe(EXPECTED_FEE);
    });

    test("Rejects when provided fee_destination does not match stored value", async () => {
        const customWallet = testHelper.createUserAccount();
        await program.setRedemptionFeeDestination({ feeDestination: customWallet.publicKey });

        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            amount: REDEMPTION_AMOUNT
        });

        const offer = await program.getRedemptionOffer(onycMint, usdcMint);
        const counter = offer.requestCounter.toNumber() - 1;
        const redemptionRequestPda = program.getRedemptionRequestPda(redemptionOfferPda, counter);

        // Pass a different wallet as fee destination — should be rejected
        const wrongWallet = testHelper.createUserAccount();
        await expect(
            program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: redemptionRequestPda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint,
                feeDestination: wrongWallet.publicKey
            })
        ).rejects.toThrow("Invalid fee destination account");
    });

    test("No fee transfer when feeBasisPoints is 0", async () => {
        // Reset fee to 0
        await program.updateRedemptionOfferFee({
            redemptionOffer: redemptionOfferPda,
            newFeeBasisPoints: 0
        });

        await createAndFulfill();

        const feeVaultPda = program.pdas.redemptionFeeVaultAuthorityPda;
        const feeVaultAta = getAssociatedTokenAddressSync(onycMint, feeVaultPda, true);
        let balance: bigint;
        try {
            balance = await testHelper.getTokenAccountBalance(feeVaultAta);
        } catch {
            balance = BigInt(0);
        }
        expect(Number(balance)).toBe(0);
    });

    test("1% fee on 1 ONyc: fee=10_000_000, net=990_000_000, user receives 990_000 USDC units", async () => {
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            amount: REDEMPTION_AMOUNT
        });

        const offer = await program.getRedemptionOffer(onycMint, usdcMint);
        const counter = offer.requestCounter.toNumber() - 1;
        const redemptionRequestPda = program.getRedemptionRequestPda(redemptionOfferPda, counter);

        await program.fulfillRedemptionRequest({
            offer: offerPda,
            redemptionOffer: redemptionOfferPda,
            redemptionRequest: redemptionRequestPda,
            redeemer: redeemer.publicKey,
            redemptionAdmin,
            tokenInMint: onycMint,
            tokenOutMint: usdcMint
        });

        // Fee vault PDA ATA should have exactly EXPECTED_FEE
        const feeVaultPda = program.pdas.redemptionFeeVaultAuthorityPda;
        const feeVaultAta = getAssociatedTokenAddressSync(onycMint, feeVaultPda, true);
        const feeBalance = await testHelper.getTokenAccountBalance(feeVaultAta);
        expect(Number(feeBalance)).toBe(EXPECTED_FEE);

        // User USDC ATA should have EXPECTED_TOKEN_OUT
        const userUsdcAta = getAssociatedTokenAddressSync(usdcMint, redeemer.publicKey, false);
        const userBalance = await testHelper.getTokenAccountBalance(userUsdcAta);
        expect(Number(userBalance)).toBe(EXPECTED_TOKEN_OUT);
    });
});
