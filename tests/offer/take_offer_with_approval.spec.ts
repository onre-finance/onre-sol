import { Keypair, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program";
import { Ed25519Helper } from "../helpers/ed25519_helper";

describe("Take Offer With Approval", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;
    let user: Keypair;
    let trustedAuthority: Keypair;
    let userTokenOutAccount: PublicKey;
    let offerId: number;

    beforeAll(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints
        tokenInMint = testHelper.createMint(6); // USDC-like (6 decimals)
        tokenOutMint = testHelper.createMint(9); // ONyc-like (9 decimals)

        // Create trusted authority keypair
        trustedAuthority = testHelper.createUserAccount();

        // Initialize program and offers
        await program.initialize();
        await program.initializeOffers();

        // Set trusted authority
        await program.setTrustedAccount({
            trusted: trustedAuthority.publicKey
        });

        // Create an offer that requires approval
        await program.makeOffer({
            tokenInMint,
            tokenOutMint,
            withApproval: true
        });

        const offerAccount = await program.getOfferAccount();
        const offer = offerAccount.offers.find(o => o.offerId.toNumber() !== 0);
        offerId = offer.offerId.toNumber();

        // Initialize vault authority
        await program.initializeVaultAuthority();

        // Create token accounts
        user = testHelper.createUserAccount();
        testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(10_000e6), true);
        testHelper.createTokenAccount(tokenInMint, testHelper.getBoss(), BigInt(0)); // Boss token in account
        testHelper.createTokenAccount(tokenOutMint, testHelper.getBoss(), BigInt(10_000e9));
        userTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, user.publicKey);

        // Create and fund vault
        testHelper.createTokenAccount(tokenInMint, program.pdas.offerVaultAuthorityPda, BigInt(0), true);
        testHelper.createTokenAccount(tokenOutMint, program.pdas.offerVaultAuthorityPda, BigInt(0), true);

        // Fund vault
        await program.offerVaultDeposit({
            amount: 10_000e9,
            tokenMint: tokenOutMint
        });

        // Add vector to the offer
        const currentTime = await testHelper.getCurrentClockTime();
        await program.addOfferVector({
            offerId,
            startTime: currentTime,
            startPrice: 1e9, // 1.0 with 9 decimals
            apr: 36_500, // 3.65% APR
            priceFixDuration: 86400 // 1 day
        });
    });

    it("Should fail to take offer requiring approval without approval message", async () => {
        // Try to take offer without approval - should fail with ApprovalRequired error
        await expect(
            program.takeOffer({
                offerId,
                tokenInAmount: 1_000_100,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            })
        ).rejects.toThrow("ApprovalRequired");
    });

    it("Should successfully take offer with valid Ed25519 approval signature", async () => {
        // Use the helper to execute the approved take offer
        await Ed25519Helper.executeApprovedTakeOffer({
            program,
            offerId,
            tokenInAmount: 1_000_100,
            tokenInMint,
            tokenOutMint,
            user: user.publicKey,
            userKeypair: user,
            trustedAuthority,
            boss: testHelper.getBoss()
        });

        // Verify user received tokens
        const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
        expect(userTokenOutBalance).toBe(BigInt(1e9));
    });

    it("Should fail with expired approval message", async () => {
        const expiredTime = Math.floor(Date.now() / 1000) - 3600; // 1 hour ago

        await expect(
            Ed25519Helper.executeApprovedTakeOffer({
                program,
                offerId,
                tokenInAmount: 1_000_100,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                userKeypair: user,
                trustedAuthority,
                boss: testHelper.getBoss(),
                expiryTime: expiredTime
            })
        ).rejects.toThrow();
    });
});