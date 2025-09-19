import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { getAssociatedTokenAddressSync, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { OnreProgram } from "../onre_program.ts";

const MAX_OFFERS = 10;

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

        await program.initialize();
        await program.initializeOffers();
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
        const offerAccount = await program.getOfferAccount();

        expect(offerAccount.counter.toNumber()).toBe(1);

        const firstOffer = offerAccount.offers[0];
        expect(firstOffer.offerId.toNumber()).toBe(1);
        expect(firstOffer.tokenInMint.toString()).toBe(tokenInMint.toString());
        expect(firstOffer.tokenOutMint.toString()).toBe(tokenOutMint.toString());
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
        let offerAccount = await program.getOfferAccount();

        expect(offerAccount.counter.toNumber()).toBe(2);

        // Find offers by their auto-generated IDs
        const firstOffer = await program.getOffer(1);
        expect(firstOffer).toBeDefined();
        expect(firstOffer!.tokenOutMint.toString()).toBe(token1Out.toString());

        const secondOffer = await program.getOffer(2);
        expect(secondOffer).toBeDefined();
        expect(secondOffer!.tokenOutMint.toString()).toBe(token2Out.toString());
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

    test("Make more than max offers should fail", async () => {
        // Create MAX_OFFERS offers
        for (let i = 0; i < MAX_OFFERS; i++) {
            // Create unique mints for each offer to avoid duplicate transaction issues
            const uniqueTokenIn = testHelper.createMint(9);
            const uniqueTokenOut = testHelper.createMint(9);

            await program.makeOffer({
                tokenInMint: uniqueTokenIn,
                tokenOutMint: uniqueTokenOut
            });
        }

        // Verify array is full
        const offerAccount = await program.getOfferAccount();
        const activeOffers = offerAccount.offers.filter(offer => offer.offerId.toNumber() > 0).length;
        expect(activeOffers).toBe(MAX_OFFERS);

        // when - try to make one more offer (should fail)
        await expect(program.makeOffer({
            tokenInMint,
            tokenOutMint
        })).rejects.toThrow("Offer account is full, cannot create more offers");
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
        const offerAccount = await program.getOfferAccount();

        expect(offerAccount.counter.toNumber()).toBe(1);

        const firstOffer = offerAccount.offers[0];
        expect(firstOffer.offerId.toNumber()).toBe(1);
        expect(firstOffer.tokenInMint.toString()).toBe(token2022Mint.toString());
        expect(firstOffer.tokenOutMint.toString()).toBe(tokenOutMint.toString());
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
        const offerAccount = await program.getOfferAccount();

        expect(offerAccount.counter.toNumber()).toBe(1);

        const firstOffer = offerAccount.offers[0];
        expect(firstOffer.offerId.toNumber()).toBe(1);
        expect(firstOffer.tokenInMint.toString()).toBe(tokenInMint.toString());
        expect(firstOffer.tokenOutMint.toString()).toBe(token2022Mint.toString());
    });
});