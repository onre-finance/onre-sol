import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper.ts";
import { OnreProgram } from "../onre_program.ts";

describe("Initialize Kill Switch", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let nonBoss: Keypair;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        nonBoss = testHelper.createUserAccount();
    });

    test("Kill switch is initialized with correct default values when state is initialized", async () => {
        // when
        await program.initialize({ onycMint: testHelper.createMint(9) });

        // then
        const state = await program.getState();
        expect(state.isKilled).toBe(false);
        expect(state.boss.toString()).toBe(testHelper.getBoss().toString());
    });

    test("Non-boss signer cannot initialize state with boss account mismatch - should fail", async () => {
        // when & then - try to initialize with nonBoss signing but boss account set to different pubkey
        await expect(
            program.program.methods
                .initialize()
                .accounts({
                    boss: testHelper.getBoss() // boss account is the real boss
                })
                .signers([nonBoss]) // but nonBoss is trying to sign
                .rpc()
        ).rejects.toThrow();
    });

    test("Cannot initialize state twice", async () => {
        // given
        await program.initialize({ onycMint: testHelper.createMint(9) });

        // when & then
        await expect(
            program.initialize({ onycMint: testHelper.createMint(9) })
        ).rejects.toThrow();
    });

    test("State is initialized with correct default values including kill switch", async () => {
        // when
        await program.initialize({ onycMint: testHelper.createMint(9) });

        // then
        const state = await program.getState();
        expect(state.isKilled).toBe(false);
        expect(state.boss.toString()).toBe(testHelper.getBoss().toString());
        // All admin slots should be empty (default PublicKey)
        for (const admin of state.admins) {
            expect(admin.toString()).toBe(PublicKey.default.toString());
        }
    });
});