import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { getAssociatedTokenAddressSync, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { OnreProgram } from "../onre_program.ts";

describe("Make redemption offer", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let usdcMint: PublicKey;
    let onycMint: PublicKey;
    let offerPda: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        // Create mints
        usdcMint = testHelper.createMint(6);
        onycMint = testHelper.createMint(9);

        await program.initialize({ onycMint });

        // Create the base offer (USDC -> ONyc)
        await program.makeOffer({
            tokenInMint: usdcMint,
            tokenOutMint: onycMint
        });

        offerPda = program.getOfferPda(usdcMint, onycMint);
    });

    test("Make redemption offer by boss should succeed", async () => {
        // when
        await program.makeRedemptionOffer({
            offer: offerPda
        });

        // then
        const redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);

        expect(redemptionOffer.offer.toString()).toBe(offerPda.toString());
        expect(redemptionOffer.tokenInMint.toString()).toBe(onycMint.toString());
        expect(redemptionOffer.tokenOutMint.toString()).toBe(usdcMint.toString());
        expect(redemptionOffer.executedRedemptions.toString()).toBe("0");
        expect(redemptionOffer.requestedRedemptions.toString()).toBe("0");
    });

    test("Make redemption offer by redemption_admin should succeed", async () => {
        // given
        const redemptionAdmin = testHelper.createUserAccount();
        await program.setRedemptionAdmin({ redemptionAdmin: redemptionAdmin.publicKey });

        // when
        await program.makeRedemptionOffer({
            offer: offerPda,
            signer: redemptionAdmin
        });

        // then
        const redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        expect(redemptionOffer.offer.toString()).toBe(offerPda.toString());
    });

    test("Make redemption offer should initialize vault token_in account (ONyc)", async () => {
        // when
        await program.makeRedemptionOffer({
            offer: offerPda
        });

        // then
        await expect(
            testHelper.getAccount(
                getAssociatedTokenAddressSync(
                    onycMint,
                    program.pdas.redemptionVaultAuthorityPda,
                    true
                )
            )
        ).resolves.toBeDefined();
    });

    test("Make redemption offer should initialize vault token_out account (USDC)", async () => {
        // when
        await program.makeRedemptionOffer({
            offer: offerPda
        });

        // then
        await expect(
            testHelper.getAccount(
                getAssociatedTokenAddressSync(
                    usdcMint,
                    program.pdas.redemptionVaultAuthorityPda,
                    true
                )
            )
        ).resolves.toBeDefined();
    });

    test("Should reject when called by unauthorized user", async () => {
        // given
        const unauthorizedUser = testHelper.createUserAccount();

        // when/then
        await expect(
            program.makeRedemptionOffer({
                offer: offerPda,
                signer: unauthorizedUser
            })
        ).rejects.toThrow();
    });

    test("Should reject duplicate redemption offers", async () => {
        // given - Create first redemption offer
        await program.makeRedemptionOffer({
            offer: offerPda
        });

        // when/then - Try to create same redemption offer again
        await expect(
            program.makeRedemptionOffer({
                offer: offerPda
            })
        ).rejects.toThrow();
    });

    test("Should reject when offer PDA is invalid", async () => {
        // given - Use a random public key as offer
        const invalidOfferPda = new PublicKey(
            "11111111111111111111111111111111"
        );

        // when/then
        await expect(
            program.makeRedemptionOffer({
                offer: invalidOfferPda
            })
        ).rejects.toThrow();
    });

    test("Should work with Token2022 as token_in (ONyc as Token2022)", async () => {
        // given - Create a new ONyc mint as Token2022
        const onyc2022 = testHelper.createMint2022(9);
        await program.setOnycMint({ onycMint: onyc2022 });

        await program.makeOffer({
            tokenInMint: usdcMint,
            tokenOutMint: onyc2022
        });
        const offer2022Pda = program.getOfferPda(usdcMint, onyc2022);

        // when
        await program.makeRedemptionOffer({
            offer: offer2022Pda,
            tokenInProgram: TOKEN_2022_PROGRAM_ID
        });

        // then
        const redemptionOffer = await program.getRedemptionOffer(
            onyc2022,
            usdcMint
        );
        expect(redemptionOffer.tokenInMint.toString()).toBe(
            onyc2022.toString()
        );
        expect(redemptionOffer.tokenOutMint.toString()).toBe(
            usdcMint.toString()
        );
    });

    test("Should create multiple redemption offers for different token pairs", async () => {
        // given - Create offers for multiple token pairs
        const usdt = testHelper.createMint(6);
        const dai = testHelper.createMint(18);

        await program.makeOffer({
            tokenInMint: usdt,
            tokenOutMint: onycMint
        });
        await program.makeOffer({
            tokenInMint: dai,
            tokenOutMint: onycMint
        });

        const usdtOfferPda = program.getOfferPda(usdt, onycMint);
        const daiOfferPda = program.getOfferPda(dai, onycMint);

        // when
        await program.makeRedemptionOffer({ offer: offerPda });
        await program.makeRedemptionOffer({ offer: usdtOfferPda });
        await program.makeRedemptionOffer({ offer: daiOfferPda });

        // then
        const redemptionOffer1 = await program.getRedemptionOffer(
            onycMint,
            usdcMint
        );
        const redemptionOffer2 = await program.getRedemptionOffer(onycMint, usdt);
        const redemptionOffer3 = await program.getRedemptionOffer(onycMint, dai);

        expect(redemptionOffer1.tokenOutMint.toString()).toBe(
            usdcMint.toString()
        );
        expect(redemptionOffer2.tokenOutMint.toString()).toBe(usdt.toString());
        expect(redemptionOffer3.tokenOutMint.toString()).toBe(dai.toString());
    });

    test("Should verify redemption offer references correct offer", async () => {
        // when
        await program.makeRedemptionOffer({
            offer: offerPda
        });

        // then
        const redemptionOffer = await program.getRedemptionOffer(onycMint, usdcMint);
        const originalOffer = await program.getOffer(usdcMint, onycMint);

        expect(redemptionOffer.offer.toString()).toBe(offerPda.toString());
        expect(redemptionOffer.tokenInMint.toString()).toBe(
            originalOffer.tokenOutMint.toString()
        );
        expect(redemptionOffer.tokenOutMint.toString()).toBe(
            originalOffer.tokenInMint.toString()
        );
    });
});
