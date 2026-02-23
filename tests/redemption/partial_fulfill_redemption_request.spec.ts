import { Keypair, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Partial fulfill redemption request", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let usdcMint: PublicKey;
    let onycMint: PublicKey;
    let offerPda: PublicKey;
    let redemptionOfferPda: PublicKey;
    let redemptionAdmin: Keypair;
    let redeemer: Keypair;

    // 9 ONyc total (9 decimals); split across partial calls using distinct amounts to avoid
    // the "AlreadyProcessed" bankrun deduplication issue (identical tx bytes are rejected).
    const REDEMPTION_AMOUNT = 9_000_000_000; // 9 ONyc
    const FIRST_PARTIAL   = 2_000_000_000;   // 2 ONyc
    const SECOND_PARTIAL  = 3_000_000_000;   // 3 ONyc
    const THIRD_PARTIAL   = 4_000_000_000;   // 4 ONyc (FIRST+SECOND+THIRD == REDEMPTION_AMOUNT)

    // At 1:1 price with 9 vs 6 decimals: N ONyc → N/1000 USDC (base units)
    const onycToUsdc = (onyc: number) => onyc / 1_000;

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
            basePrice: 1e9,   // 1.0 (9 decimals) → 1 ONyc = 1 USDC
            apr: 0,
            priceFixDuration: 86400
        });

        await program.makeRedemptionOffer({ offer: offerPda });
        redemptionOfferPda = program.getRedemptionOfferPda(onycMint, usdcMint);

        // Give both mint authorities to the program for burn + mint
        await program.transferMintAuthorityToProgram({ mint: onycMint });
        await program.transferMintAuthorityToProgram({ mint: usdcMint });

        // Pre-create boss token accounts (required by the instruction)
        const boss = testHelper.getBoss();
        testHelper.createTokenAccount(onycMint, boss, BigInt(0), true);
        testHelper.createTokenAccount(usdcMint, boss, BigInt(0), true);

        // Fund redeemer with ONyc
        testHelper.createTokenAccount(onycMint, redeemer.publicKey, BigInt(10_000e9), true);

        // Create the redemption request for REDEMPTION_AMOUNT
        await program.createRedemptionRequest({
            redemptionOffer: redemptionOfferPda,
            redeemer,
            amount: REDEMPTION_AMOUNT
        });
    });

    describe("Partial fulfillment state", () => {
        test("Should update fulfilled_amount after partial fulfill", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);

            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: pda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint,
                amount: new BN(FIRST_PARTIAL)
            });

            const request = await program.program.account.redemptionRequest.fetch(pda);
            expect(request.fulfilledAmount.toString()).toBe(FIRST_PARTIAL.toString());
            expect(request.amount.toString()).toBe(REDEMPTION_AMOUNT.toString());
        });

        test("Should keep account open after partial fulfill", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);

            await program.fulfillRedemptionRequest({
                offer: offerPda,
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: pda,
                redeemer: redeemer.publicKey,
                redemptionAdmin,
                tokenInMint: onycMint,
                tokenOutMint: usdcMint,
                amount: new BN(FIRST_PARTIAL)
            });

            const request = await program.program.account.redemptionRequest.fetch(pda);
            expect(request).toBeDefined();
        });

        test("Should close account only after final fulfillment", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);

            // Three distinct amounts so bankrun doesn't deduplicate the transactions
            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(FIRST_PARTIAL)
            });
            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(SECOND_PARTIAL)
            });
            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(THIRD_PARTIAL)
            });

            // After the last call the account must be closed
            await expect(
                program.program.account.redemptionRequest.fetch(pda)
            ).rejects.toThrow();
        });
    });

    describe("Token accounting", () => {
        test("Should transfer correct token_out amount for each partial call", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);
            const userUsdcAccount = getAssociatedTokenAddressSync(usdcMint, redeemer.publicKey);

            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(FIRST_PARTIAL)
            });
            expect(await testHelper.getTokenAccountBalance(userUsdcAccount))
                .toBe(BigInt(onycToUsdc(FIRST_PARTIAL)));

            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(SECOND_PARTIAL)
            });
            expect(await testHelper.getTokenAccountBalance(userUsdcAccount))
                .toBe(BigInt(onycToUsdc(FIRST_PARTIAL + SECOND_PARTIAL)));

            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(THIRD_PARTIAL)
            });
            expect(await testHelper.getTokenAccountBalance(userUsdcAccount))
                .toBe(BigInt(onycToUsdc(REDEMPTION_AMOUNT)));
        });

        test("Should decrement requested_redemptions by fulfilled amount each call", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);

            const offerBefore = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            const requestedBefore = BigInt(offerBefore.requestedRedemptions.toString());

            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(FIRST_PARTIAL)
            });

            const offerAfter = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            expect(BigInt(offerAfter.requestedRedemptions.toString()))
                .toBe(requestedBefore - BigInt(FIRST_PARTIAL));
        });

        test("Should increment executed_redemptions by fulfilled amount each call", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);

            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(FIRST_PARTIAL)
            });

            const offer = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            expect(offer.executedRedemptions.toString()).toBe(FIRST_PARTIAL.toString());
        });
    });

    describe("Offer-level accounting after full 3-call redemption", () => {
        test("Should have correct executed_redemptions after full redemption in three calls", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);

            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(FIRST_PARTIAL)
            });
            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(SECOND_PARTIAL)
            });
            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(THIRD_PARTIAL)
            });

            const offer = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            expect(offer.executedRedemptions.toString()).toBe(REDEMPTION_AMOUNT.toString());
            expect(offer.requestedRedemptions.toString()).toBe("0");
        });
    });

    describe("Cancellation of partially fulfilled requests", () => {
        test("Should return only remaining (unfulfilled) tokens on cancel", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);
            const userOnycAccount = getAssociatedTokenAddressSync(onycMint, redeemer.publicKey);

            // Capture balance after request creation (REDEMPTION_AMOUNT is locked in vault)
            const onycBefore = await testHelper.getTokenAccountBalance(userOnycAccount);

            // Partially fulfill FIRST_PARTIAL (burned from vault, not from user account)
            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(FIRST_PARTIAL)
            });

            // Cancel: only the remaining (REDEMPTION_AMOUNT - FIRST_PARTIAL) is returned
            await program.cancelRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: pda,
                signer: redeemer,
                redemptionAdmin: redemptionAdmin.publicKey
            });

            const onycAfter = await testHelper.getTokenAccountBalance(userOnycAccount);
            // FIRST_PARTIAL was burned; remaining (SECOND+THIRD) was returned from vault
            const returned = BigInt(REDEMPTION_AMOUNT - FIRST_PARTIAL);
            expect(onycAfter).toBe(onycBefore + returned);

            // Account should be closed
            await expect(
                program.program.account.redemptionRequest.fetch(pda)
            ).rejects.toThrow();
        });

        test("Should have correct requested_redemptions after cancel of partially fulfilled request", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);

            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(FIRST_PARTIAL)
            });

            await program.cancelRedemptionRequest({
                redemptionOffer: redemptionOfferPda,
                redemptionRequest: pda,
                signer: redeemer,
                redemptionAdmin: redemptionAdmin.publicKey
            });

            const offer = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            expect(offer.requestedRedemptions.toString()).toBe("0");
        });
    });

    describe("Validation errors", () => {
        test("Should reject amount of zero", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);

            await expect(
                program.fulfillRedemptionRequest({
                    offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                    redeemer: redeemer.publicKey, redemptionAdmin,
                    tokenInMint: onycMint, tokenOutMint: usdcMint,
                    amount: new BN(0)
                })
            ).rejects.toThrow("Invalid amount: must be greater than zero");
        });

        test("Should reject amount exceeding remaining unfulfilled balance", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);

            await expect(
                program.fulfillRedemptionRequest({
                    offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                    redeemer: redeemer.publicKey, redemptionAdmin,
                    tokenInMint: onycMint, tokenOutMint: usdcMint,
                    amount: new BN(REDEMPTION_AMOUNT + 1)
                })
            ).rejects.toThrow("Amount exceeds remaining unfulfilled balance");
        });

        test("Should reject second call when amount exceeds remaining balance", async () => {
            const pda = program.getRedemptionRequestPda(redemptionOfferPda, 0);

            // Fulfill FIRST + SECOND, leaving only THIRD remaining
            await program.fulfillRedemptionRequest({
                offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                redeemer: redeemer.publicKey, redemptionAdmin,
                tokenInMint: onycMint, tokenOutMint: usdcMint,
                amount: new BN(FIRST_PARTIAL + SECOND_PARTIAL)
            });

            // Attempt to fulfill more than the remaining THIRD_PARTIAL
            await expect(
                program.fulfillRedemptionRequest({
                    offer: offerPda, redemptionOffer: redemptionOfferPda, redemptionRequest: pda,
                    redeemer: redeemer.publicKey, redemptionAdmin,
                    tokenInMint: onycMint, tokenOutMint: usdcMint,
                    amount: new BN(THIRD_PARTIAL + 1)
                })
            ).rejects.toThrow("Amount exceeds remaining unfulfilled balance");
        });
    });
});
