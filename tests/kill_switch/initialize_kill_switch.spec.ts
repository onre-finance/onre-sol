import { PublicKey, Keypair } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Initialize Kill Switch", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let nonBoss: Keypair;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        nonBoss = testHelper.createUserAccount();

        // Initialize state first
        await program.initialize();
    });

    test("Boss can initialize kill switch state successfully", async () => {
        // when
        await program.initializeKillSwitchState();

        // then
        const killSwitchState = await program.getKillSwitchState();
        expect(killSwitchState.isKilled).toBe(false);
    });

    test("Non-boss cannot initialize kill switch state - should fail", async () => {
        // when & then
        await expect(
            program.program.methods
                .initializeKillSwitchState()
                .accounts({
                    state: program.statePda,
                })
                .signers([nonBoss])
                .rpc()
        ).rejects.toThrow();
    });

    test("Cannot initialize kill switch state twice", async () => {
        // given
        await program.initializeKillSwitchState();

        // when & then
        await expect(
            program.initializeKillSwitchState()
        ).rejects.toThrow();
    });

    test("Kill switch state is initialized with correct default values", async () => {
        // when
        await program.initializeKillSwitchState();

        // then
        const killSwitchState = await program.getKillSwitchState();
        expect(killSwitchState.isKilled).toBe(false);
    });
});