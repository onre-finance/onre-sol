import { Keypair, PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper.ts";
import { OnreProgram } from "../onre_program.ts";

describe("Remove Admin", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let nonBoss: Keypair;
    let admin1: Keypair;
    let admin2: Keypair;
    let admin3: Keypair;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        nonBoss = testHelper.createUserAccount();
        admin1 = testHelper.createUserAccount();
        admin2 = testHelper.createUserAccount();
        admin3 = testHelper.createUserAccount();

        // Initialize state (includes admin array initialization)
        await program.initialize({ onycMint: testHelper.createMint(9) });

        // Add some admins for testing
        await program.addAdmin({ admin: admin1.publicKey });
        await program.addAdmin({ admin: admin2.publicKey });
        await program.addAdmin({ admin: admin3.publicKey });
    });

    test("Boss can remove an admin successfully", async () => {
        // given
        const initialState = await program.getState();
        expect(initialState.admins).toContainEqual(admin2.publicKey);
        const initialActiveAdmins = initialState.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(initialActiveAdmins).toHaveLength(3);

        // when
        await program.removeAdmin({ admin: admin2.publicKey });

        // then
        const state = await program.getState();
        expect(state.admins).not.toContainEqual(admin2.publicKey);
        const activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(2);
        // Verify other admins are still there
        expect(state.admins).toContainEqual(admin1.publicKey);
        expect(state.admins).toContainEqual(admin3.publicKey);
    });

    test("Non-boss cannot remove admin - should fail", async () => {
        // when & then
        await expect(
            program.removeAdmin({ admin: admin1.publicKey, signer: nonBoss })
        ).rejects.toThrow();
    });

    test("Cannot remove admin that doesn't exist", async () => {
        // given
        const nonExistentAdmin = testHelper.createUserAccount();

        // when & then
        await expect(
            program.removeAdmin({ admin: nonExistentAdmin.publicKey })
        ).rejects.toThrow();
    });

    test("Can remove all admins one by one", async () => {
        // given
        const initialState = await program.getState();
        const initialActiveAdmins = initialState.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(initialActiveAdmins).toHaveLength(3);

        // when - remove first admin
        await program.removeAdmin({ admin: admin1.publicKey });

        // then
        let state = await program.getState();
        let activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(2);
        expect(state.admins).not.toContainEqual(admin1.publicKey);

        // when - remove second admin
        await program.removeAdmin({ admin: admin2.publicKey });

        // then
        state = await program.getState();
        activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(1);
        expect(state.admins).not.toContainEqual(admin2.publicKey);

        // when - remove third admin
        await program.removeAdmin({ admin: admin3.publicKey });

        // then
        state = await program.getState();
        activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(0);
        expect(state.admins).not.toContainEqual(admin3.publicKey);
    });

    test("Can remove and re-add the same admin", async () => {
        // given - remove admin1
        await program.removeAdmin({ admin: admin1.publicKey });

        let state = await program.getState();
        expect(state.admins).not.toContainEqual(admin1.publicKey);
        let activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(2);

        // when - re-add admin1
        await program.addAdmin({ admin: admin1.publicKey });

        // then
        state = await program.getState();
        expect(state.admins).toContainEqual(admin1.publicKey);
        activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(3);
    });

    test("Boss can remove themselves if they are in the admin list", async () => {
        // given - add boss as admin
        await program.addAdmin({ admin: testHelper.getBoss() });

        let state = await program.getState();
        expect(state.admins).toContainEqual(testHelper.getBoss());
        let activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(4);

        // when - boss removes themselves
        await program.removeAdmin({ admin: testHelper.getBoss() });

        // then
        state = await program.getState();
        expect(state.admins).not.toContainEqual(testHelper.getBoss());
        activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(3);
    });

    test("Cannot remove the same admin twice", async () => {
        // given - remove admin1 first time
        await program.removeAdmin({ admin: admin1.publicKey });

        // when & then - try to remove the same admin again
        await expect(
            program.removeAdmin({ admin: admin1.publicKey })
        ).rejects.toThrow();
    });

    test("Removing admin preserves order of remaining admins", async () => {
        // given - verify initial order
        const initialState = await program.getState();
        expect(initialState.admins[0]).toEqual(admin1.publicKey);
        expect(initialState.admins[1]).toEqual(admin2.publicKey);
        expect(initialState.admins[2]).toEqual(admin3.publicKey);

        // when - remove middle admin
        await program.removeAdmin({ admin: admin2.publicKey });

        // then - verify remaining admins (swap_remove moves last element to removed position)
        const state = await program.getState();
        const activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(2);
        expect(state.admins).toContainEqual(admin1.publicKey);
        expect(state.admins).toContainEqual(admin3.publicKey);
        expect(state.admins).not.toContainEqual(admin2.publicKey);
    });
});