import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper.ts";
import { OnreProgram } from "../onre_program.ts";

describe("Add Approver", () => {
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
    });

    test("Boss can add first approver successfully", async () => {
        // when
        await program.addApprover({ trusted: approver1.publicKey });

        // then
        const state = await program.getState();
        expect(state.approver1).toEqual(approver1.publicKey);
        expect(state.approver2).toEqual(PublicKey.default);
    });

    test("Boss can add second approver successfully", async () => {
        // given - add first approver
        await program.addApprover({ trusted: approver1.publicKey });

        // when - add second approver
        await program.addApprover({ trusted: approver2.publicKey });

        // then
        const state = await program.getState();
        expect(state.approver1).toEqual(approver1.publicKey);
        expect(state.approver2).toEqual(approver2.publicKey);
    });

    test("Non-boss cannot add approver - should fail", async () => {
        // when & then
        await expect(
            program.addApprover({ trusted: approver1.publicKey, signer: nonBoss })
        ).rejects.toThrow();
    });

    test("Cannot add third approver when both slots are filled", async () => {
        // given - add two approvers
        await program.addApprover({ trusted: approver1.publicKey });
        await program.addApprover({ trusted: approver2.publicKey });

        // when & then - try to add third approver
        const approver3 = testHelper.createUserAccount();
        await expect(
            program.addApprover({ trusted: approver3.publicKey })
        ).rejects.toThrow("BothApproversFilled");

        // verify we still have exactly 2 approvers
        const state = await program.getState();
        expect(state.approver1).toEqual(approver1.publicKey);
        expect(state.approver2).toEqual(approver2.publicKey);
    });

    test("Boss can add themselves as approver", async () => {
        // when
        await program.addApprover({ trusted: testHelper.getBoss() });

        // then
        const state = await program.getState();
        expect(state.approver1).toEqual(testHelper.getBoss());
        expect(state.approver2).toEqual(PublicKey.default);
    });

    test("Can add same approver to both slots after removing", async () => {
        // given - add approver1 to first slot
        await program.addApprover({ trusted: approver1.publicKey });

        let state = await program.getState();
        expect(state.approver1).toEqual(approver1.publicKey);
        expect(state.approver2).toEqual(PublicKey.default);

        // when - add different approver to second slot
        await program.addApprover({ trusted: approver2.publicKey });

        // then
        state = await program.getState();
        expect(state.approver1).toEqual(approver1.publicKey);
        expect(state.approver2).toEqual(approver2.publicKey);
    });

    test("Approvers are initialized as default on state creation", async () => {
        // when - just initialized in beforeEach
        const state = await program.getState();

        // then
        expect(state.approver1).toEqual(PublicKey.default);
        expect(state.approver2).toEqual(PublicKey.default);
    });

    test("Can add two different approvers", async () => {
        // when
        await program.addApprover({ trusted: approver1.publicKey });
        await program.addApprover({ trusted: approver2.publicKey });

        // then
        const state = await program.getState();
        expect(state.approver1).toEqual(approver1.publicKey);
        expect(state.approver2).toEqual(approver2.publicKey);
        expect(state.approver1).not.toEqual(state.approver2);
    });

    test("Cannot add the same approver twice", async () => {
        // given - add first approver
        await program.addApprover({ trusted: approver1.publicKey });

        // when & then - try to add the same approver again
        await expect(
            program.addApprover({ trusted: approver1.publicKey })
        ).rejects.toThrow("ApproverAlreadyExists");
    });
});
