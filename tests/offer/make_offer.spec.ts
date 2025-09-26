import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { getAssociatedTokenAddressSync, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { OnreProgram } from "../onre_program.ts";

describe("Make offer", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints
        tokenInMint = testHelper.createMint(9);
        tokenOutMint = testHelper.createMint(9);

        await program.initialize({ onycMint: tokenOutMint });
    });

    test("Make an offer should succeed", async () => {
        // when
        const feeBasisPoints = 500; // 5% fee
        await program.makeOffer({
            tokenInMint,
            tokenOutMint,
            feeBasisPoints
        });

        // then
        const offer = await program.getOffer(tokenInMint, tokenOutMint);

        expect(offer.tokenInMint.toString()).toBe(tokenInMint.toString());
        expect(offer.tokenOutMint.toString()).toBe(tokenOutMint.toString());
        expect(offer.feeBasisPoints).toBe(feeBasisPoints);
    });

    test("Make multiple offers should succeed", async () => {
        // when
        // make first offer
        const token1In = testHelper.createMint(9);
        const token1Out = testHelper.createMint(9);

        await program.makeOffer({
            tokenInMint: token1In,
            tokenOutMint: token1Out
        });

        const token2In = testHelper.createMint(9);
        const token2Out = testHelper.createMint(9);

        // make second offer
        await program.makeOffer({
            tokenInMint: token2In,
            tokenOutMint: token2Out
        });

        // then
        // Each offer should exist as a separate PDA
        const firstOffer = await program.getOffer(token1In, token1Out);
        expect(firstOffer).toBeDefined();
        expect(firstOffer.tokenInMint.toString()).toBe(token1In.toString());
        expect(firstOffer.tokenOutMint.toString()).toBe(token1Out.toString());

        const secondOffer = await program.getOffer(token2In, token2Out);
        expect(secondOffer).toBeDefined();
        expect(secondOffer.tokenInMint.toString()).toBe(token2In.toString());
        expect(secondOffer.tokenOutMint.toString()).toBe(token2Out.toString());
    });

    test("Make an offer should initialize vault token_in account", async () => {
        // when
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        // then
        await expect(testHelper.getAccount(
            getAssociatedTokenAddressSync(tokenInMint, program.pdas.offerVaultAuthorityPda, true))
        ).resolves.toBeDefined();
    });

    test("Make an offer with invalid token mints should fail", async () => {
        // when
        await expect(program.makeOffer({
            tokenInMint: new PublicKey(0),
            tokenOutMint: new PublicKey(0)
        })).rejects.toThrow();
    });

    test("Should reject duplicate offers", async () => {
        // Create first offer
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        // Try to create same offer again - should fail
        await expect(program.makeOffer({
            tokenInMint,
            tokenOutMint
        })).rejects.toThrow();
    });

    test("Should reject when called by non-boss", async () => {
        // Create an offer first
        await expect(program.makeOffer({
            tokenInMint,
            tokenOutMint,
            signer: testHelper.createUserAccount()
        })).rejects.toThrow("unknown signer");
    });

    test("Should accept Token2022 as token_in_mint", async () => {
        // Create a Token2022 mint
        const token2022Mint = testHelper.createMint2022(9);

        // when
        await program.makeOffer({
            tokenInMint: token2022Mint,
            tokenOutMint,
            tokenInProgram: TOKEN_2022_PROGRAM_ID
        });

        // then
        const offer = await program.getOffer(token2022Mint, tokenOutMint);

        expect(offer.tokenInMint.toString()).toBe(token2022Mint.toString());
        expect(offer.tokenOutMint.toString()).toBe(tokenOutMint.toString());
    });

    test("Should accept Token2022 as token_out_mint", async () => {
        // Create a Token2022 mint
        const token2022Mint = testHelper.createMint2022(9);

        // when
        await program.makeOffer({
            tokenInMint,
            tokenOutMint: token2022Mint
        });

        // then
        const offer = await program.getOffer(tokenInMint, token2022Mint);

        expect(offer.tokenInMint.toString()).toBe(tokenInMint.toString());
        expect(offer.tokenOutMint.toString()).toBe(token2022Mint.toString());
    });
});