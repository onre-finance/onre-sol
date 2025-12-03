import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper.ts";
import { OnreProgram } from "../onre_program.ts";

describe("Remove Approver", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let nonBoss: Keypair;
    let approver1: Keypair;
    let approver2: Keypair;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        nonBoss = testHelper.createUserAccount();
        approver1 = testHelper.createUserAccount();
        approver2 = testHelper.createUserAccount();

        // Initialize state
        await program.initialize({ onycMint: testHelper.createMint(9) });

        // Add two approvers for testing
        await program.addApprover({ trusted: approver1.publicKey });
        await program.addApprover({ trusted: approver2.publicKey });
    });

    test("Boss can remove first approver successfully", async () => {
        // given
        const initialState = await program.getState();
        expect(initialState.approver1).toEqual(approver1.publicKey);
        expect(initialState.approver2).toEqual(approver2.publicKey);

        // when
        await program.removeApprover({ approver: approver1.publicKey });

        // then
        const state = await program.getState();
        expect(state.approver1).toEqual(PublicKey.default);
        expect(state.approver2).toEqual(approver2.publicKey);
    });

    test("Boss can remove second approver successfully", async () => {
        // given
        const initialState = await program.getState();
        expect(initialState.approver1).toEqual(approver1.publicKey);
        expect(initialState.approver2).toEqual(approver2.publicKey);

        // when
        await program.removeApprover({ approver: approver2.publicKey });

        // then
        const state = await program.getState();
        expect(state.approver1).toEqual(approver1.publicKey);
        expect(state.approver2).toEqual(PublicKey.default);
    });

    test("Non-boss cannot remove approver - should fail", async () => {
        // when & then
        await expect(
            program.removeApprover({ approver: approver1.publicKey, signer: nonBoss })
        ).rejects.toThrow();
    });

    test("Cannot remove approver that doesn't exist", async () => {
        // given
        const nonExistentApprover = testHelper.createUserAccount();

        // when & then
        await expect(
            program.removeApprover({ approver: nonExistentApprover.publicKey })
        ).rejects.toThrow("NotAnApprover");
    });

    test("Cannot remove default pubkey", async () => {
        // when & then
        await expect(
            program.removeApprover({ approver: PublicKey.default })
        ).rejects.toThrow("InvalidApprover");
    });

    test("Can remove both approvers", async () => {
        // given
        const initialState = await program.getState();
        expect(initialState.approver1).toEqual(approver1.publicKey);
        expect(initialState.approver2).toEqual(approver2.publicKey);

        // when - remove first approver
        await program.removeApprover({ approver: approver1.publicKey });

        // then
        let state = await program.getState();
        expect(state.approver1).toEqual(PublicKey.default);
        expect(state.approver2).toEqual(approver2.publicKey);

        // when - remove second approver
        await program.removeApprover({ approver: approver2.publicKey });

        // then
        state = await program.getState();
        expect(state.approver1).toEqual(PublicKey.default);
        expect(state.approver2).toEqual(PublicKey.default);
    });

    test("Can remove and re-add the same approver", async () => {
        // given - remove approver1
        await program.removeApprover({ approver: approver1.publicKey });

        let state = await program.getState();
        expect(state.approver1).toEqual(PublicKey.default);
        expect(state.approver2).toEqual(approver2.publicKey);

        await testHelper.advanceSlot();

        // when - re-add approver1
        await program.addApprover({ trusted: approver1.publicKey });

        // then - it should be added to the first empty slot (approver1)
        state = await program.getState();
        expect(state.approver1).toEqual(approver1.publicKey);
        expect(state.approver2).toEqual(approver2.publicKey);
    });

    test("Boss can remove themselves if they are an approver", async () => {
        // given - remove approver1 and add boss
        await program.removeApprover({ approver: approver1.publicKey });
        await program.addApprover({ trusted: testHelper.getBoss() });

        let state = await program.getState();
        expect(state.approver1).toEqual(testHelper.getBoss());
        expect(state.approver2).toEqual(approver2.publicKey);

        // when - boss removes themselves
        await program.removeApprover({ approver: testHelper.getBoss() });

        // then
        state = await program.getState();
        expect(state.approver1).toEqual(PublicKey.default);
        expect(state.approver2).toEqual(approver2.publicKey);
    });

    test("Cannot remove the same approver twice", async () => {
        // given - remove approver1 first time
        await program.removeApprover({ approver: approver1.publicKey });

        await testHelper.advanceSlot();

        // when & then - try to remove the same approver again
        await expect(
            program.removeApprover({ approver: approver1.publicKey })
        ).rejects.toThrow("NotAnApprover");
    });

    test("After removing an approver, can add a new one to the empty slot", async () => {
        // given - remove approver2
        await program.removeApprover({ approver: approver2.publicKey });

        let state = await program.getState();
        expect(state.approver1).toEqual(approver1.publicKey);
        expect(state.approver2).toEqual(PublicKey.default);

        // when - add new approver
        const newApprover = testHelper.createUserAccount();
        await program.addApprover({ trusted: newApprover.publicKey });

        // then - new approver should be in approver2 slot
        state = await program.getState();
        expect(state.approver1).toEqual(approver1.publicKey);
        expect(state.approver2).toEqual(newApprover.publicKey);
    });

    test("Removing approver from slot 1 allows adding to that slot", async () => {
        // given - remove approver1
        await program.removeApprover({ approver: approver1.publicKey });

        let state = await program.getState();
        expect(state.approver1).toEqual(PublicKey.default);
        expect(state.approver2).toEqual(approver2.publicKey);

        // when - add new approver (should go to first empty slot)
        const newApprover = testHelper.createUserAccount();
        await program.addApprover({ trusted: newApprover.publicKey });

        // then - new approver should be in approver1 slot
        state = await program.getState();
        expect(state.approver1).toEqual(newApprover.publicKey);
        expect(state.approver2).toEqual(approver2.publicKey);
    });

    test("Can swap approvers by removing and adding", async () => {
        // given
        const initialState = await program.getState();
        expect(initialState.approver1).toEqual(approver1.publicKey);
        expect(initialState.approver2).toEqual(approver2.publicKey);

        // when - remove both and add in reverse order
        await program.removeApprover({ approver: approver1.publicKey });
        await program.removeApprover({ approver: approver2.publicKey });

        await testHelper.advanceSlot();

        await program.addApprover({ trusted: approver2.publicKey });
        await program.addApprover({ trusted: approver1.publicKey });

        // then - order should be swapped
        const state = await program.getState();
        expect(state.approver1).toEqual(approver2.publicKey);
        expect(state.approver2).toEqual(approver1.publicKey);
    });
});
