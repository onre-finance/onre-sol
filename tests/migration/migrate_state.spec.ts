import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Migrate State", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let nonBoss: Keypair;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        nonBoss = testHelper.createUserAccount();

        // Initialize the program with old state structure (only boss field)
        await program.initialize();
    });

    test("Boss can migrate state successfully", async () => {
        // when - boss migrates the state
        await program.migrateState();

        // then - state should be migrated and accessible
        const state = await program.getState();
        expect(state.boss).toEqual(testHelper.getBoss());
        expect(state.isKilled).toBe(false); // Should be initialized to false
    });

    test("Non-boss cannot migrate state - should fail", async () => {
        // when & then
        await expect(
            program.migrateState({ signer: nonBoss })
        ).rejects.toThrow("Boss pubkey mismatch");
    });

    test("Migration is idempotent - can be called multiple times", async () => {
        // given - first migration
        await program.migrateState();
        const stateAfterFirst = await program.getState();
        expect(stateAfterFirst.isKilled).toBe(false);

        // when - migrate again
        await program.migrateState();

        // then - state should still be accessible and unchanged
        const stateAfterSecond = await program.getState();
        expect(stateAfterSecond.boss).toEqual(testHelper.getBoss());
        expect(stateAfterSecond.isKilled).toBe(false);
    });

    test("Migration preserves existing boss field", async () => {
        // given - get original boss
        const originalBoss = testHelper.getBoss();

        // when - migrate state
        await program.migrateState();

        // then - boss should be preserved
        const state = await program.getState();
        expect(state.boss).toEqual(originalBoss);
    });

    test("After migration, kill switch functionality works", async () => {
        // given - migrate state first
        await program.migrateState();

        // Initialize admin state for kill switch functionality
        await program.initializeAdminState();
        const admin = testHelper.createUserAccount();
        await program.addAdmin({ admin: admin.publicKey });

        // when - enable kill switch
        await program.killSwitch({ enable: true, signer: admin });

        // then - state should reflect kill switch enabled
        const state = await program.getState();
        expect(state.isKilled).toBe(true);

        // when - disable kill switch (only boss can disable)
        await program.killSwitch({ enable: false });

        // then - state should reflect kill switch disabled
        const finalState = await program.getState();
        expect(finalState.isKilled).toBe(false);
    });

    test("State account maintains proper size after migration", async () => {
        // given - get initial account size
        const stateAccountInfo = await testHelper.getAccountInfo(program.statePda);
        const initialSize = stateAccountInfo.data.length;

        // when - migrate state
        await program.migrateState();

        // then - account size should be maintained (already includes is_killed in new deployments)
        const migratedAccountInfo = await testHelper.getAccountInfo(program.statePda);
        const finalSize = migratedAccountInfo.data.length;

        expect(finalSize).toBe(initialSize); // Size should be the same for new deployments
        expect(finalSize).toBe(41); // 8 bytes discriminator + 32 bytes boss + 1 byte is_killed
    });

    test("Migration preserves kill switch state when enabled before migration", async () => {
        // given - initialize admin state and enable kill switch before migration
        await program.initializeAdminState();
        const admin = testHelper.createUserAccount();
        await program.addAdmin({ admin: admin.publicKey });

        // Enable kill switch before migration
        await program.killSwitch({ enable: true, signer: admin });

        // Verify kill switch is enabled before migration
        const stateBefore = await program.getState();
        expect(stateBefore.isKilled).toBe(true);

        // when - migrate state
        await program.migrateState();

        // then - kill switch state should be preserved
        const stateAfter = await program.getState();
        expect(stateAfter.boss).toEqual(testHelper.getBoss());
        expect(stateAfter.isKilled).toBe(true); // Should still be enabled after migration

        // Also verify kill switch functionality still works after migration
        await program.killSwitch({ enable: false }); // Only boss can disable
        const finalState = await program.getState();
        expect(finalState.isKilled).toBe(false);
    });
});