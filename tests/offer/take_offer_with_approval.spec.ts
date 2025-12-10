import { Keypair, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program";

describe("Take Offer With Approval", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;
    let user: Keypair;
    let approver: Keypair;
    let userTokenOutAccount: PublicKey;

    beforeAll(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints
        tokenInMint = testHelper.createMint(6); // USDC-like (6 decimals)
        tokenOutMint = testHelper.createMint(9); // ONyc-like (9 decimals)

        // Create approver keypair
        approver = testHelper.createUserAccount();

        // Initialize program and offers
        await program.initialize({ onycMint: tokenOutMint });

        // Set approver
        await program.addApprover({
            trusted: approver.publicKey
        });

        // Create an offer that requires approval
        await program.makeOffer({
            tokenInMint,
            tokenOutMint,
            withApproval: true
        });

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
            baseTime: currentTime,
            basePrice: 1e9, // 1.0 with 9 decimals
            apr: 36_500, // 3.65% APR
            priceFixDuration: 86400, // 1 day,
            tokenInMint: tokenInMint,
            tokenOutMint: tokenOutMint
        });
    });

    it("Should fail to take offer requiring approval without approver signer", async () => {
        // Try to take offer without approver - should fail with MissingApproverSignature error
        await expect(
            program.takeOffer({
                tokenInAmount: 1_000_100,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user
            })
        ).rejects.toThrow("Missing approver signature");
    });

    it("Should successfully take offer with valid approver signature", async () => {
        // Take offer with approver signer
        await program.takeOffer({
            tokenInAmount: 1_000_100,
            tokenInMint,
            tokenOutMint,
            user: user.publicKey,
            signer: user,
            approver: approver
        });

        // Verify user received tokens
        const userTokenOutBalance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
        expect(userTokenOutBalance).toBe(BigInt(1e9));
    });

    it("Should fail with invalid approver signature", async () => {
        // Create a different keypair that is not registered as approver
        const invalidApprover = testHelper.createUserAccount();

        await expect(
            program.takeOffer({
                tokenInAmount: 1_000_100,
                tokenInMint,
                tokenOutMint,
                user: user.publicKey,
                signer: user,
                approver: invalidApprover
            })
        ).rejects.toThrow("Invalid approver signature");
    });
});