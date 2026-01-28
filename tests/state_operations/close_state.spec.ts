import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper.ts";
import { OnreProgram } from "../onre_program.ts";

describe("Close State", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let nonBoss: Keypair;
    let onycMint: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        nonBoss = testHelper.createUserAccount();
        onycMint = testHelper.createMint(9);

        // Initialize state
        await program.initialize({ onycMint });
    });

    test("Boss can close state successfully", async () => {
        // given - state exists
        const initialState = await program.getState();
        expect(initialState.boss).toEqual(testHelper.getBoss());

        // when
        await program.closeState();

        // then - state account should no longer exist
        await expect(program.getState()).rejects.toThrow();
    });

    test("Non-boss cannot close state - should fail", async () => {
        // when & then
        await expect(
            program.closeState({ signer: nonBoss })
        ).rejects.toThrow();

        // verify state still exists
        const state = await program.getState();
        expect(state.boss).toEqual(testHelper.getBoss());
    });

    test("After closing state, it can be re-initialized", async () => {
        // given - close the state
        await program.closeState();
        await expect(program.getState()).rejects.toThrow();

        // when - re-initialize
        const newOnycMint = testHelper.createMint(9);
        await program.initialize({ onycMint: newOnycMint });

        // then - state exists again
        const newState = await program.getState();
        expect(newState.boss).toEqual(testHelper.getBoss());
        expect(newState.onycMint).toEqual(newOnycMint);
    });

    test("After closing and re-initializing, state has correct default values", async () => {
        // given - close the state
        await program.closeState();

        // when - re-initialize with new mint
        const newOnycMint = testHelper.createMint(9);
        await program.initialize({ onycMint: newOnycMint });

        // then - verify all fields are properly initialized
        const state = await program.getState();
        expect(state.boss).toEqual(testHelper.getBoss());
        expect(state.onycMint).toEqual(newOnycMint);
        expect(state.isKilled).toBe(false);
        expect(state.proposedBoss).toEqual(PublicKey.default);
        expect(state.maxSupply.toNumber()).toEqual(0);

        // Verify admins array is empty (all default pubkeys)
        const activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(0);

        // Verify approvers are default
        expect(state.approver1).toEqual(PublicKey.default);
        expect(state.approver2).toEqual(PublicKey.default);
    });
});
