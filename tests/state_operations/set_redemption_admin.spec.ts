import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Set Redemption Admin", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let redemptionAdmin: Keypair;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        redemptionAdmin = testHelper.createUserAccount();

        // Initialize state
        await program.initialize({ onycMint: testHelper.createMint(9) });
    });

    test("Redemption admin can be set", async () => {
        // given
        const initialState = await program.getState();
        expect(initialState.redemptionAdmin).toEqual(PublicKey.default);

        // when
        await program.setRedemptionAdmin({ redemptionAdmin: redemptionAdmin.publicKey });

        // then
        const state = await program.getState();
        expect(state.redemptionAdmin).toEqual(redemptionAdmin.publicKey);
    });

    test("Redemption admin can be unset (set to default Pubkey)", async () => {
        // given - first set a redemption admin
        await program.setRedemptionAdmin({ redemptionAdmin: redemptionAdmin.publicKey });
        let state = await program.getState();
        expect(state.redemptionAdmin).toEqual(redemptionAdmin.publicKey);

        // when - unset by setting to default pubkey
        await program.setRedemptionAdmin({ redemptionAdmin: PublicKey.default });

        // then
        state = await program.getState();
        expect(state.redemptionAdmin).toEqual(PublicKey.default);
    });
});
