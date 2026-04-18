import { BN } from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Prop AMM", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;
    let user: Keypair;
    let userTokenInAccount: PublicKey;
    let userTokenOutAccount: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        tokenInMint = testHelper.createMint(6);
        tokenOutMint = testHelper.createMint(9);

        await program.initialize({ onycMint: tokenOutMint });
        await program.makeOffer({
            tokenInMint,
            tokenOutMint,
        });
        await program.setMainOffer({
            offer: program.getOfferPda(tokenInMint, tokenOutMint),
        });

        user = testHelper.createUserAccount();
        userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(10_000e6), true);
        userTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(0), true);

        testHelper.createTokenAccount(tokenInMint, testHelper.getBoss(), BigInt(0));
        testHelper.createTokenAccount(tokenOutMint, testHelper.getBoss(), BigInt(10_000e9));
        testHelper.createTokenAccount(tokenInMint, program.pdas.offerVaultAuthorityPda, BigInt(0), true);
        testHelper.createTokenAccount(tokenOutMint, program.pdas.offerVaultAuthorityPda, BigInt(0), true);

        await program.offerVaultDeposit({
            amount: 10_000e9,
            tokenMint: tokenOutMint,
        });
    });

    it("Should return quote data and allow open_swap with minimum_out", async () => {
        const currentTime = await testHelper.getCurrentClockTime();

        await program.addOfferVector({
            tokenInMint,
            tokenOutMint,
            baseTime: currentTime,
            basePrice: 1e9,
            apr: 0,
            priceFixDuration: 86400,
        });

        const quote = await program.quoteSwap({
            tokenInAmount: 1_000_000,
            tokenInMint,
            tokenOutMint,
            quoteExpiry: currentTime + 60,
        });

        expect(quote.tokenInAmount.toNumber()).toBe(1_000_000);
        expect(quote.tokenOutAmount.toString()).toBe("1000000000");
        expect(quote.minimumOut.eq(quote.tokenOutAmount)).toBe(true);

        await program.openSwap({
            tokenInAmount: 1_000_000,
            minimumOut: quote.minimumOut,
            quoteExpiry: quote.quoteExpiry,
            tokenInMint,
            tokenOutMint,
            user: user.publicKey,
            signer: user,
        });

        const userTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);
        expect(userTokenOutBalanceAfter).toBe(BigInt(1e9));
    });

    it("Should reject open_swap when minimum_out is not met or quote is expired", async () => {
        const currentTime = await testHelper.getCurrentClockTime();

        await program.addOfferVector({
            tokenInMint,
            tokenOutMint,
            baseTime: currentTime,
            basePrice: 1e9,
            apr: 0,
            priceFixDuration: 86400,
        });

        const quote = await program.quoteSwap({
            tokenInAmount: 1_000_000,
            tokenInMint,
            tokenOutMint,
            quoteExpiry: currentTime + 60,
        });

        await expect(program.openSwap({
            tokenInAmount: 1_000_000,
            minimumOut: quote.minimumOut.addn(1),
            quoteExpiry: quote.quoteExpiry,
            tokenInMint,
            tokenOutMint,
            user: user.publicKey,
            signer: user,
        })).rejects.toThrow();

        const shortQuote = await program.quoteSwap({
            tokenInAmount: 1_000_000,
            tokenInMint,
            tokenOutMint,
            quoteExpiry: currentTime + 1,
        });

        await testHelper.advanceClockBy(2);

        await expect(program.openSwap({
            tokenInAmount: 1_000_000,
            minimumOut: shortQuote.minimumOut,
            quoteExpiry: shortQuote.quoteExpiry,
            tokenInMint,
            tokenOutMint,
            user: user.publicKey,
            signer: user,
        })).rejects.toThrow();
    });

    it("Should support sell-side open_swap using the canonical offer", async () => {
        const currentTime = await testHelper.getCurrentClockTime();

        await program.transferMintAuthorityToProgram({
            mint: tokenOutMint,
        });

        await program.addOfferVector({
            tokenInMint,
            tokenOutMint,
            baseTime: currentTime,
            basePrice: 1e9,
            apr: 0,
            priceFixDuration: 86400,
        });

        testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(2e9), true);
        testHelper.createTokenAccount(tokenInMint, program.pdas.redemptionVaultAuthorityPda, BigInt(10_000e6), true);
        testHelper.createTokenAccount(tokenOutMint, program.pdas.redemptionVaultAuthorityPda, BigInt(0), true);

        const quote = await program.quoteSwap({
            tokenInAmount: 1e9,
            tokenInMint: tokenOutMint,
            tokenOutMint: tokenInMint,
            quoteExpiry: currentTime + 60,
        });

        expect(quote.offer.toString()).toBe(program.getOfferPda(tokenInMint, tokenOutMint).toString());
        expect(quote.tokenOutAmount.toString()).toBe("1000000");

        const vaultBefore = await testHelper.getTokenAccountBalance(program.getRedemptionVaultAta(tokenInMint));

        await program.openSwap({
            tokenInAmount: 1e9,
            minimumOut: quote.minimumOut,
            quoteExpiry: quote.quoteExpiry,
            tokenInMint: tokenOutMint,
            tokenOutMint: tokenInMint,
            user: user.publicKey,
            signer: user,
        });

        const userUsdcAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);
        expect(userUsdcAfter).toBe(BigInt(10_001e6));

        const vaultAfter = await testHelper.getTokenAccountBalance(program.getRedemptionVaultAta(tokenInMint));
        expect(vaultAfter).toBe(vaultBefore - BigInt(1e6));
    });
});
