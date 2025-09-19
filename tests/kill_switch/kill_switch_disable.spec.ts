import { PublicKey, Keypair } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Kill Switch Disable", () => {
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

        // Enable kill switch initially for disable tests
        await program.killSwitch({ enable: true });
    });

    test("Boss can disable kill switch successfully", async () => {
        // given - kill switch is enabled
        const initialState = await program.getKillSwitchState();
        expect(initialState.isKilled).toBe(true);

        // when
        await program.killSwitch({ enable: false });

        // then
        const finalState = await program.getKillSwitchState();
        expect(finalState.isKilled).toBe(false);
    });

    test("Admin cannot disable kill switch - should fail", async () => {
        // given - kill switch is enabled
        const initialState = await program.getKillSwitchState();
        expect(initialState.isKilled).toBe(true);

        // when & then
        await expect(
            program.killSwitch({ enable: false, signer: admin })
        ).rejects.toThrow("Only boss can disable the kill switch");

        // verify kill switch is still enabled
        const finalState = await program.getKillSwitchState();
        expect(finalState.isKilled).toBe(true);
    });

    test("Non-boss and non-admin cannot disable kill switch - should fail", async () => {
        // given - kill switch is enabled
        const initialState = await program.getKillSwitchState();
        expect(initialState.isKilled).toBe(true);

        // when & then
        await expect(
            program.killSwitch({ enable: false, signer: nonBoss })
        ).rejects.toThrow("Only boss can disable the kill switch");

        // verify kill switch is still enabled
        const finalState = await program.getKillSwitchState();
        expect(finalState.isKilled).toBe(true);
    });

    test("Can disable kill switch when already enabled", async () => {
        // given - kill switch is enabled
        const initialState = await program.getKillSwitchState();
        expect(initialState.isKilled).toBe(true);

        // when
        await program.killSwitch({ enable: false });

        // then
        const finalState = await program.getKillSwitchState();
        expect(finalState.isKilled).toBe(false);
    });

    test("Boss can disable kill switch after admin enabled it", async () => {
        // given - kill switch is already enabled from beforeEach
        // verify it's enabled
        const initialState = await program.getKillSwitchState();
        expect(initialState.isKilled).toBe(true);

        // when - boss disables
        await program.killSwitch({ enable: false });

        // then
        const finalState = await program.getKillSwitchState();
        expect(finalState.isKilled).toBe(false);
    });

    test("Only boss privilege is required for disable, not admin privilege", async () => {
        // given - remove admin privileges
        await program.removeAdmin({ admin: admin.publicKey });

        // verify admin no longer has privileges to enable
        await expect(
            program.killSwitch({ enable: true, signer: admin })
        ).rejects.toThrow("Unauthorized to enable the kill switch");

        // when - boss can still disable
        await program.killSwitch({ enable: false });

        // then
        const finalState = await program.getKillSwitchState();
        expect(finalState.isKilled).toBe(false);
    });
});