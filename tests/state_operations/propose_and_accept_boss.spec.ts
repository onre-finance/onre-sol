import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper.ts";
import { OnreProgram } from "../onre_program.ts";

describe("Propose and Accept Boss", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let newBoss: Keypair;
    let nonBoss: Keypair;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        newBoss = testHelper.createUserAccount();
        nonBoss = testHelper.createUserAccount();

        // Initialize state
        await program.initialize({ onycMint: testHelper.createMint(9) });
    });

    test("Boss can propose a new boss successfully", async () => {
        // when
        await program.proposeBoss({ newBoss: newBoss.publicKey });

        // then
        const state = await program.getState();
        expect(state.proposedBoss).toEqual(newBoss.publicKey);
        expect(state.boss).toEqual(testHelper.getBoss()); // boss unchanged
    });

    test("Proposed boss can accept and become the new boss", async () => {
        // given
        await program.proposeBoss({ newBoss: newBoss.publicKey });

        // when
        await program.acceptBoss({ newBoss });

        // then
        const state = await program.getState();
        expect(state.boss).toEqual(newBoss.publicKey);
        expect(state.proposedBoss).toEqual(PublicKey.default); // proposal cleared
    });

    test("Complete two-step transfer workflow", async () => {
        // given
        const oldBoss = testHelper.getBoss();

        // when - step 1: propose
        await program.proposeBoss({ newBoss: newBoss.publicKey });

        // verify intermediate state
        let state = await program.getState();
        expect(state.boss).toEqual(oldBoss);
        expect(state.proposedBoss).toEqual(newBoss.publicKey);

        // when - step 2: accept
        await program.acceptBoss({ newBoss });

        // then
        state = await program.getState();
        expect(state.boss).toEqual(newBoss.publicKey);
        expect(state.proposedBoss).toEqual(PublicKey.default);
    });

    test("Non-boss cannot propose a new boss", async () => {
        // when & then
        await expect(
            program.proposeBoss({ newBoss: newBoss.publicKey, signer: nonBoss })
        ).rejects.toThrow();
    });

    test("Cannot accept boss proposal without a proposal", async () => {
        // when & then
        await expect(
            program.acceptBoss({ newBoss })
        ).rejects.toThrow("NoBossProposal");
    });

    test("Wrong account cannot accept boss proposal", async () => {
        // given
        await program.proposeBoss({ newBoss: newBoss.publicKey });

        // when & then
        await expect(
            program.acceptBoss({ newBoss: nonBoss })
        ).rejects.toThrow("NotProposedBoss");
    });

    test("Current boss cannot accept the proposal (must be proposed boss)", async () => {
        // given
        await program.proposeBoss({ newBoss: newBoss.publicKey });

        // when & then - current boss tries to accept
        await expect(
            program.acceptBoss({ newBoss: testHelper.context.payer })
        ).rejects.toThrow("NotProposedBoss");
    });

    test("Cannot propose default (system program) address as boss", async () => {
        // when & then
        await expect(
            program.proposeBoss({ newBoss: PublicKey.default })
        ).rejects.toThrow("InvalidBossAddress");
    });

    test("Boss can change proposal before acceptance", async () => {
        // given
        const firstProposedBoss = testHelper.createUserAccount();
        const secondProposedBoss = testHelper.createUserAccount();

        // when
        await program.proposeBoss({ newBoss: firstProposedBoss.publicKey });
        await program.proposeBoss({ newBoss: secondProposedBoss.publicKey });

        // then
        const state = await program.getState();
        expect(state.proposedBoss).toEqual(secondProposedBoss.publicKey);
    });

    test("First proposed boss cannot accept after proposal is changed", async () => {
        // given
        const firstProposedBoss = testHelper.createUserAccount();
        const secondProposedBoss = testHelper.createUserAccount();
        await program.proposeBoss({ newBoss: firstProposedBoss.publicKey });
        await program.proposeBoss({ newBoss: secondProposedBoss.publicKey });

        // when & then
        await expect(
            program.acceptBoss({ newBoss: firstProposedBoss })
        ).rejects.toThrow("NotProposedBoss");

        // but second one can accept
        await program.acceptBoss({ newBoss: secondProposedBoss });
        const state = await program.getState();
        expect(state.boss).toEqual(secondProposedBoss.publicKey);
    });

    test("New boss can propose another transfer", async () => {
        // given - complete first transfer
        await program.proposeBoss({ newBoss: newBoss.publicKey });
        await program.acceptBoss({ newBoss });

        // when - new boss proposes third boss
        const thirdBoss = testHelper.createUserAccount();
        await program.proposeBoss({
            newBoss: thirdBoss.publicKey,
            signer: newBoss
        });

        // then
        const state = await program.getState();
        expect(state.boss).toEqual(newBoss.publicKey);
        expect(state.proposedBoss).toEqual(thirdBoss.publicKey);

        // and third boss can accept
        await program.acceptBoss({ newBoss: thirdBoss });
        const finalState = await program.getState();
        expect(finalState.boss).toEqual(thirdBoss.publicKey);
    });
});
