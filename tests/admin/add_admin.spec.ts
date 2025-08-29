import { PublicKey, Keypair } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

describe("Add Admin", () => {
    let testHelper: TestHelper;
    let program: Program<Onreapp>;
    let boss: PublicKey;
    let adminStatePda: PublicKey;
    let nonBoss: Keypair;
    let newAdmin: Keypair;

    beforeEach(async () => {
        const programInfo: AddedProgram = {
            programId: ONREAPP_PROGRAM_ID,
            name: "onreapp"
        };

        const workspace = process.cwd();
        const context = await startAnchor(workspace, [programInfo], []);

        const provider = new BankrunProvider(context);
        program = new Program<Onreapp>(idl, provider);
        testHelper = new TestHelper(context, program);

        boss = provider.wallet.publicKey;
        nonBoss = testHelper.createUserAccount();
        newAdmin = testHelper.createUserAccount();

        [adminStatePda] = PublicKey.findProgramAddressSync(
            [Buffer.from("admin_state")],
            ONREAPP_PROGRAM_ID
        );

        // Initialize state and admin state
        await program.methods.initialize().accounts({ boss }).rpc();
        await program.methods.initializeAdminState().accounts({
            state: testHelper.statePda
        }).rpc();
    });

    test("Boss can add a new admin successfully", async () => {
        // when
        await program.methods.addAdmin(newAdmin.publicKey).rpc();

        // then
        const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).toContainEqual(newAdmin.publicKey);
        const activeAdmins = adminStateAccount.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(1);
    });

    test("Non-boss cannot add admin - should fail", async () => {
        // when & then
        await expect(
            program.methods.addAdmin(newAdmin.publicKey).signers([nonBoss]).rpc()
        ).rejects.toThrow();
    });

    test("Cannot add the same admin twice", async () => {
        // given
        await program.methods.addAdmin(newAdmin.publicKey).rpc();

        // when & then
        await expect(
            program.methods.addAdmin(newAdmin.publicKey).rpc()
        ).rejects.toThrow();
    });

    test("Cannot add more than 20 admins", async () => {
        // given - add 20 admins
        const admins = [];
        for (let i = 0; i < 20; i++) {
            const admin = testHelper.createUserAccount();
            admins.push(admin);
            await program.methods.addAdmin(admin.publicKey).rpc();
        }

        // when & then - try to add 21st admin
        const extraAdmin = testHelper.createUserAccount();
        await expect(
            program.methods.addAdmin(extraAdmin.publicKey).rpc()
        ).rejects.toThrow();

        // verify we still have exactly 20 admins
        const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        const activeAdmins = adminStateAccount.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(20);
    });

    test("Can add multiple different admins", async () => {
        // given
        const admin1 = testHelper.createUserAccount();
        const admin2 = testHelper.createUserAccount();
        const admin3 = testHelper.createUserAccount();

        // when
        await program.methods.addAdmin(admin1.publicKey).rpc();
        await program.methods.addAdmin(admin2.publicKey).rpc();
        await program.methods.addAdmin(admin3.publicKey).rpc();

        // then
        const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).toContainEqual(admin1.publicKey);
        expect(adminStateAccount.admins).toContainEqual(admin2.publicKey);
        expect(adminStateAccount.admins).toContainEqual(admin3.publicKey);
        const activeAdmins = adminStateAccount.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(3);
    });

    test("Boss can add themselves as admin", async () => {
        // when
        await program.methods.addAdmin(boss).rpc();

        // then
        const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).toContainEqual(boss);
        const activeAdmins = adminStateAccount.admins.filter(admin => !admin.equals(PublicKey.default));
        expect(activeAdmins).toHaveLength(1);
    });
});