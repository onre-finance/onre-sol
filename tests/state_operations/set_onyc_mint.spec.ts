import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Set ONyc Mint", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let originalOnycMint: PublicKey;
    let newOnycMint: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        originalOnycMint = testHelper.createMint(9);
        newOnycMint = testHelper.createMint(9);

        // Initialize state with original ONyc mint
        await program.initialize({ onycMint: originalOnycMint });
    });

    test("Boss can successfully update ONyc mint", async () => {
        // when
        await program.setOnycMint({ onycMint: newOnycMint });

        // then
        const state = await program.getState();
        expect(state.onycMint).toEqual(newOnycMint);
        expect(state.boss).toEqual(testHelper.getBoss());
    });

    test("Non-boss cannot update ONyc mint - should fail", async () => {
        // when & then
        await expect(
            program.setOnycMint({ onycMint: newOnycMint, signer: testHelper.createUserAccount() })
        ).rejects.toThrow();
    });

    test("Setting ONyc mint preserves other state fields", async () => {
        // given - add an admin to verify it's preserved
        const admin = testHelper.createUserAccount();
        await program.addAdmin({ admin: admin.publicKey });

        const stateBefore = await program.getState();
        const originalBoss = stateBefore.boss;
        const originalIsKilled = stateBefore.isKilled;
        const originalAdmins = stateBefore.admins;

        // when - set new ONyc mint
        await program.setOnycMint({ onycMint: newOnycMint });

        // then - other fields should be preserved
        const stateAfter = await program.getState();
        expect(stateAfter.boss).toEqual(originalBoss);
        expect(stateAfter.isKilled).toEqual(originalIsKilled);
        expect(stateAfter.admins).toEqual(originalAdmins);
        expect(stateAfter.onycMint).toEqual(newOnycMint);
    });

    test("Can use Token-2022 program mint", async () => {
        // given - create a mint using Token-2022 program
        const token2022Mint = testHelper.createMint2022(9);

        // when - set ONyc mint to Token-2022 mint
        await program.setOnycMint({ onycMint: token2022Mint });

        // then - should succeed
        const state = await program.getState();
        expect(state.onycMint).toEqual(token2022Mint);
    });

    test("Multiple ONyc mint updates work correctly", async () => {
        // given - create multiple mints
        const mint1 = testHelper.createMint(6);
        const mint2 = testHelper.createMint(9);
        const mint3 = testHelper.createMint(18);

        // when - update ONyc mint multiple times
        await program.setOnycMint({ onycMint: mint1 });
        let state = await program.getState();
        expect(state.onycMint).toEqual(mint1);

        await program.setOnycMint({ onycMint: mint2 });
        state = await program.getState();
        expect(state.onycMint).toEqual(mint2);

        await program.setOnycMint({ onycMint: mint3 });
        state = await program.getState();
        expect(state.onycMint).toEqual(mint3);
    });

    test("Setting ONyc mint after kill switch operations works", async () => {
        // given - set up kill switch functionality
        const admin = testHelper.createUserAccount();
        await program.addAdmin({ admin: admin.publicKey });

        // Enable and disable kill switch
        await program.setKillSwitch({ enable: true, signer: admin });
        await program.setKillSwitch({ enable: false });

        // when - set new ONyc mint after kill switch operations
        await program.setOnycMint({ onycMint: newOnycMint });

        // then - should succeed
        const state = await program.getState();
        expect(state.onycMint).toEqual(newOnycMint);
        expect(state.isKilled).toBe(false);
    });

    test("State account size remains consistent after ONyc mint update", async () => {
        // given - get initial account size
        const stateAccountInfo = await testHelper.getAccountInfo(program.pdas.statePda);
        const initialSize = stateAccountInfo.data.length;

        // when - update ONyc mint
        await program.setOnycMint({ onycMint: newOnycMint });

        // then - account size should remain the same
        const updatedAccountInfo = await testHelper.getAccountInfo(program.pdas.statePda);
        const finalSize = updatedAccountInfo.data.length;

        expect(finalSize).toBe(initialSize);
        // 8 bytes discriminator + 32 bytes boss + 32 bytes proposed_boss + 1 byte is_killed // 73 bytes
        // + 32 bytes onyc_mint + (20 * 32) bytes admins + 32 bytes approver1 + 32 bytes approver2 // 736 bytes
        // + 1 byte bump + 8 bytes max_supply + 128 bytes reserved  // 137 bytes
        expect(finalSize).toBe(946);
    });
});