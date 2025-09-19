import { PublicKey, Keypair } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Kill Switch Enable", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let nonBoss: Keypair;
    let admin: Keypair;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        nonBoss = testHelper.createUserAccount();
        admin = testHelper.createUserAccount();

        // Initialize required states
        await program.initialize();
        await program.initializeAdminState();
        await program.initializeKillSwitchState();

        // Add an admin for testing admin privileges
        await program.addAdmin({ admin: admin.publicKey });
    });

    test("Boss can enable kill switch successfully", async () => {
        // when
        await program.killSwitch({ enable: true });

        // then
        const killSwitchState = await program.getKillSwitchState();
        expect(killSwitchState.isKilled).toBe(true);
    });

    test("Admin can enable kill switch successfully", async () => {
        // when
        await program.killSwitch({ enable: true, signer: admin });

        // then
        const killSwitchState = await program.getKillSwitchState();
        expect(killSwitchState.isKilled).toBe(true);
    });

    test("Non-boss and non-admin cannot enable kill switch - should fail", async () => {
        // when & then
        await expect(
            program.killSwitch({ enable: true, signer: nonBoss })
        ).rejects.toThrow("Unauthorized to enable the kill switch");
    });

    test("Can enable kill switch when already disabled", async () => {
        // given - kill switch is initialized as disabled (false)
        const initialState = await program.getKillSwitchState();
        expect(initialState.isKilled).toBe(false);

        // when
        await program.killSwitch({ enable: true });

        // then
        const finalState = await program.getKillSwitchState();
        expect(finalState.isKilled).toBe(true);
    });

    test("Can enable kill switch when already enabled (idempotent)", async () => {
        // given - enable kill switch first
        await program.killSwitch({ enable: true });
        const initialState = await program.getKillSwitchState();
        expect(initialState.isKilled).toBe(true);

        // when - enable again (use admin signer to make transaction different)
        await program.killSwitch({ enable: true, signer: admin });

        // then - should still be enabled
        const finalState = await program.getKillSwitchState();
        expect(finalState.isKilled).toBe(true);
    });

    test("Multiple admins can enable kill switch", async () => {
        // given - add another admin
        const admin2 = testHelper.createUserAccount();
        await program.addAdmin({ admin: admin2.publicKey });

        // when - first admin enables
        await program.killSwitch({ enable: true, signer: admin });

        // then - should be enabled
        let killSwitchState = await program.getKillSwitchState();
        expect(killSwitchState.isKilled).toBe(true);

        // when - boss disables then second admin enables
        await program.killSwitch({ enable: false });
        await program.killSwitch({ enable: true, signer: admin2 });

        // then - should be enabled again
        killSwitchState = await program.getKillSwitchState();
        expect(killSwitchState.isKilled).toBe(true);
    });
});