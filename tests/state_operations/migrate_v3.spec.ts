import { Keypair, PublicKey } from "@solana/web3.js";
import { INITIAL_LAMPORTS, ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper.ts";
import { OnreProgram } from "../onre_program.ts";
import { createHash } from "crypto";

describe("Migrate State", () => {
    describe("Migration base tests", () => {
        let testHelper: TestHelper;
        let program: OnreProgram;
        let nonBoss: Keypair;

        beforeEach(async () => {
            testHelper = await TestHelper.create();
            program = new OnreProgram(testHelper.context);

            nonBoss = testHelper.createUserAccount();

            // Initialize the program with old state structure (only boss field)
            await program.initialize({ onycMint: testHelper.createMint(9) });
            await program.initializePermissionlessAuthority({ accountName: "test-account" });
        });

        test("Boss can migrate state successfully", async () => {
            // when - boss migrates the state
            await program.migrateV3();

            // then - state should be migrated and accessible
            const state = await program.getState();
            expect(state.boss).toEqual(testHelper.getBoss());
            expect(state.isKilled).toBe(false); // Should be initialized to false
        });

        test("Non-boss cannot migrate state - should fail", async () => {
            // when & then
            await expect(
                program.migrateV3(nonBoss)
            ).rejects.toThrow("Boss pubkey mismatch");
        });

        test("Migration preserves existing boss field", async () => {
            // given - get original boss
            const originalBoss = testHelper.getBoss();

            // when - migrate state
            await program.migrateV3();

            // then - boss should be preserved
            const state = await program.getState();
            expect(state.boss).toEqual(originalBoss);
        });

        test("After migration, kill switch functionality works", async () => {
            // given - migrate state first
            await program.migrateV3();

            // Admin state is now part of the main state - no separate initialization needed
            const admin = testHelper.createUserAccount();
            await program.addAdmin({ admin: admin.publicKey });

            // when - enable kill switch
            await program.setKillSwitch({ enable: true, signer: admin });

            // then - state should reflect kill switch enabled
            const state = await program.getState();
            expect(state.isKilled).toBe(true);

            // when - disable kill switch (only boss can disable)
            await program.setKillSwitch({ enable: false });

            // then - state should reflect kill switch disabled
            const finalState = await program.getState();
            expect(finalState.isKilled).toBe(false);
        });

        test("State account maintains proper size after migration", async () => {
            // given - get initial account size
            const stateAccountInfo = await testHelper.getAccountInfo(program.pdas.statePda);
            const initialSize = stateAccountInfo.data.length;

            // when - migrate state
            await program.migrateV3();

            // then - account size should be maintained (already includes is_killed in new deployments)
            const migratedAccountInfo = await testHelper.getAccountInfo(program.pdas.statePda);
            const finalSize = migratedAccountInfo.data.length;

            expect(finalSize).toBe(initialSize); // Size should be the same for new deployments
            expect(finalSize).toBe(874); // 8 bytes discriminator + 32 bytes boss + 1 byte is_killed + 32 bytes onyc_mint + (20 * 32) bytes admins + 128 reserved + 1 byte bump
        });

        test("Migration initializes kill switch to disabled and admins to empty", async () => {
            // given - state already initialized with boss
            const originalBoss = testHelper.getBoss();

            // when - migrate state
            await program.migrateV3();

            // then - migration should initialize kill switch as disabled and admins as empty
            const stateAfter = await program.getState();
            expect(stateAfter.boss).toEqual(originalBoss);
            expect(stateAfter.isKilled).toBe(false); // Should be initialized to false

            // All admin slots should be empty (default PublicKey)
            for (const admin of stateAfter.admins) {
                expect(admin.toString()).toBe(PublicKey.default.toString());
            }

            // Verify kill switch functionality works after migration
            // First add an admin
            const admin = testHelper.createUserAccount();
            await program.addAdmin({ admin: admin.publicKey });

            // Enable kill switch
            await program.setKillSwitch({ enable: true, signer: admin });
            const enabledState = await program.getState();
            expect(enabledState.isKilled).toBe(true);

            // Disable kill switch (only boss can disable)
            await program.setKillSwitch({ enable: false });
            const finalState = await program.getState();
            expect(finalState.isKilled).toBe(false);
        });
    });
    describe("Migration of old state tests", () => {
        let testHelper: TestHelper;
        let program: OnreProgram;

        beforeEach(async () => {
            testHelper = await TestHelper.create();
            program = new OnreProgram(testHelper.context);

            // Initialize the program with old state structure
            testHelper.context.setAccount(
                program.pdas.statePda,
                {
                    executable: false,
                    data: getOldStateAccountData(),
                    lamports: INITIAL_LAMPORTS,
                    owner: ONREAPP_PROGRAM_ID
                }
            );

            // Initialize the program with old permissionless authority structure
            testHelper.context.setAccount(
                program.pdas.permissionlessAuthorityPda,
                {
                    executable: false,
                    data: getOldPermissionlessAccountData(),
                    lamports: INITIAL_LAMPORTS,
                    owner: ONREAPP_PROGRAM_ID
                }
            );
        });

        test("Migration of old state and permissionless authority should work", async () => {
            // when - migrate state
            await program.migrateV3();

            // then - state should be migrated and accessible
            const state = await program.getState();
            expect(state.boss).toEqual(testHelper.getBoss()); // Boss is preserved
            expect(state.isKilled).toBe(false); // Should be initialized to false
            expect(state.onycMint.toString()).toBe(PublicKey.default.toString()); // Should be initialized to default
            expect(state.admins).toEqual(Array(20).fill(PublicKey.default)); // Should be initialized to empty array
            expect(state.approver.toString()).toBe(PublicKey.default.toString()); // Should be initialized to default
            expect(state.bump).toBeGreaterThan(0); // Should be initialized to a valid bump

            const permissionlessAuthority = await program.getPermissionlessAuthority();
            expect(permissionlessAuthority.name).toBe("ON Technologies Corporation"); // Name is preserved
            expect(permissionlessAuthority.bump).toBeGreaterThan(0); // Should be initialized to a valid bump
        });


        function accountDiscriminator(name: string) {
            return createHash("sha256").update(`account:${name}`).digest().subarray(0, 8);
        }

        function getOldStateAccountData() {
            const oldStateData = Buffer.alloc(8 + 32); // 8-byte discriminator + 32-byte boss
            oldStateData.set(accountDiscriminator("State"), 0);
            oldStateData.set(testHelper.getBoss().toBuffer(), 8);

            return oldStateData;
        }

        function getOldPermissionlessAccountData() {
            const permissionlessName = "ON Technologies Corporation";
            const nameBytes = Buffer.from(permissionlessName, "utf8");
            const len = Buffer.alloc(4);
            len.writeUInt32LE(nameBytes.length, 0);

            const permissionlessDiscriminator = accountDiscriminator("PermissionlessAccount");

            // pad to fixed body size (4 + maxLen)
            const padding = Buffer.alloc(50 - nameBytes.length, 0);

            return Buffer.concat([permissionlessDiscriminator, len, nameBytes, padding]);
        }
    });
});