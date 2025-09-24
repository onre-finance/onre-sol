import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Close offer", () => {
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

        // Initialize program and offers
        await program.initialize({ onycMint: tokenOutMint });
    });

    it("Close offer should succeed and clear the offer", async () => {
        // given - create an offer first
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        // verify offer exists
        const offer = await program.getOffer(tokenInMint, tokenOutMint);
        expect(offer).toBeDefined();

        // when - close the offer
        await program.closeOffer(tokenInMint, tokenOutMint);

        // then - verify offer is cleared
        await expect(program.getOffer(tokenInMint, tokenOutMint)).rejects.toThrow("Could not find");
    });

    it("Close offer should clear specific offer without affecting others", async () => {
        // given - create multiple offers
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        const token2In = testHelper.createMint(9);
        const token2Out = testHelper.createMint(9);

        await program.makeOffer({
            tokenInMint: token2In,
            tokenOutMint: token2Out
        });

        const token3In = testHelper.createMint(9);
        const token3Out = testHelper.createMint(9);

        await program.makeOffer({
            tokenInMint: token3In,
            tokenOutMint: token3Out
        });

        // verify all offers exist
        const offer1 = await program.getOffer(tokenInMint, tokenOutMint);
        expect(offer1.tokenInMint).toStrictEqual(tokenInMint);
        expect(offer1.tokenOutMint).toStrictEqual(tokenOutMint);

        const offer2 = await program.getOffer(token2In, token2Out);
        expect(offer2.tokenInMint).toStrictEqual(token2In);
        expect(offer2.tokenOutMint).toStrictEqual(token2Out);

        const offer3 = await program.getOffer(token3In, token3Out);
        expect(offer3.tokenInMint).toStrictEqual(token3In);
        expect(offer3.tokenOutMint).toStrictEqual(token3Out);

        // when - close the middle offer
        await program.closeOffer(token2In, token2Out);

        // then - verify only the middle offer is cleared
        await expect(program.getOffer(token2In, token2Out)).rejects.toThrow("Could not find");

        // First and third offers should still exist
        const offer1After = await program.getOffer(tokenInMint, tokenOutMint);
        expect(offer1After.tokenInMint).toStrictEqual(tokenInMint);
        expect(offer1After.tokenOutMint).toStrictEqual(tokenOutMint);
        const offer3After = await program.getOffer(token3In, token3Out);
        expect(offer3After.tokenInMint).toStrictEqual(token3In);
        expect(offer3After.tokenOutMint).toStrictEqual(token3Out);
    });

    it("Close offer with incorrect tokens should fail", async () => {
        // when/then - try to close invalid offer
        await expect(
            program.closeOffer(testHelper.createMint(9), tokenOutMint)
        ).rejects.toThrow("AnchorError caused by account: offer");

        await expect(
            program.closeOffer(tokenInMint, testHelper.createMint(9))
        ).rejects.toThrow("AnchorError caused by account: offer");
    });

    it("Close offer should fail when not called by boss", async () => {
        await program.makeOffer({
            tokenInMint,
            tokenOutMint
        });

        // when/then - try to close with different signer
        const notBoss = testHelper.createUserAccount();

        await expect(
            program.closeOffer(tokenInMint, tokenOutMint, notBoss)
        ).rejects.toThrow("unknown signer"); // Should fail due to boss constraint
    });
});