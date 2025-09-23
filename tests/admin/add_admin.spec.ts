import { PublicKey, Keypair } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Add Admin", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let nonBoss: Keypair;
    let newAdmin: Keypair;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        nonBoss = testHelper.createUserAccount();
        newAdmin = testHelper.createUserAccount();

        // Initialize state (includes admin array initialization)
        await program.initialize();
    });

    test("Boss can add a new admin successfully", async () => {
        // when
        await program.addAdmin({ admin: newAdmin.publicKey });

        // then
        const state = await program.getState();
        expect(state.admins).toContainEqual(newAdmin.publicKey);
        const activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(1);
    });

    test("Non-boss cannot add admin - should fail", async () => {
        // when & then
        await expect(
            program.addAdmin({ admin: newAdmin.publicKey, signer: nonBoss })
        ).rejects.toThrow();
    });

    test("Cannot add the same admin twice", async () => {
        // given
        await program.addAdmin({ admin: newAdmin.publicKey });

        // when & then
        await expect(
            program.addAdmin({ admin: newAdmin.publicKey })
        ).rejects.toThrow();
    });

    test("Cannot add more than 20 admins", async () => {
        // given - add 20 admins
        const admins = [];
        for (let i = 0; i < 20; i++) {
            const admin = testHelper.createUserAccount();
            admins.push(admin);
            await program.addAdmin({ admin: admin.publicKey });
        }

        // when & then - try to add 21st admin
        const extraAdmin = testHelper.createUserAccount();
        await expect(
            program.addAdmin({ admin: extraAdmin.publicKey })
        ).rejects.toThrow();

        // verify we still have exactly 20 admins
        const state = await program.getState();
        const activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(20);
    });

    test("Can add multiple different admins", async () => {
        // given
        const admin1 = testHelper.createUserAccount();
        const admin2 = testHelper.createUserAccount();
        const admin3 = testHelper.createUserAccount();

        // when
        await program.addAdmin({ admin: admin1.publicKey });
        await program.addAdmin({ admin: admin2.publicKey });
        await program.addAdmin({ admin: admin3.publicKey });

        // then
        const state = await program.getState();
        expect(state.admins).toContainEqual(admin1.publicKey);
        expect(state.admins).toContainEqual(admin2.publicKey);
        expect(state.admins).toContainEqual(admin3.publicKey);
        const activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(3);
    });

    test("Boss can add themselves as admin", async () => {
        // when
        await program.addAdmin({ admin: testHelper.getBoss() });

        // then
        const state = await program.getState();
        expect(state.admins).toContainEqual(testHelper.getBoss());
        const activeAdmins = state.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(1);
    });
});