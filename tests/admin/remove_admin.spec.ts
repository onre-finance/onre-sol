import { PublicKey, Keypair } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

describe("Remove Admin", () => {
    let testHelper: TestHelper;
    let program: Program<Onreapp>;
    let boss: PublicKey;
    let adminStatePda: PublicKey;
    let nonBoss: Keypair;
    let admin1: Keypair;
    let admin2: Keypair;
    let admin3: Keypair;

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
        admin1 = testHelper.createUserAccount();
        admin2 = testHelper.createUserAccount();
        admin3 = testHelper.createUserAccount();

        [adminStatePda] = PublicKey.findProgramAddressSync(
            [Buffer.from("admin_state")],
            ONREAPP_PROGRAM_ID
        );

        // Initialize state and admin state
        await program.methods.initialize().accounts({ boss }).rpc();
        await program.methods.initializeAdminState().accounts({
            state: testHelper.statePda
        }).rpc();

        // Add some admins for testing
        await program.methods.addAdmin(admin1.publicKey).rpc();
        await program.methods.addAdmin(admin2.publicKey).rpc();
        await program.methods.addAdmin(admin3.publicKey).rpc();
    });

    test("Boss can remove an admin successfully", async () => {
        // given
        const initialAdminState = await program.account.adminState.fetch(adminStatePda);
        expect(initialAdminState.admins).toContainEqual(admin2.publicKey);
        expect(initialAdminState.admins).toHaveLength(3);

        // when
        await program.methods.removeAdmin(admin2.publicKey).rpc();

        // then
        const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).not.toContainEqual(admin2.publicKey);
        expect(adminStateAccount.admins).toHaveLength(2);
        // Verify other admins are still there
        expect(adminStateAccount.admins).toContainEqual(admin1.publicKey);
        expect(adminStateAccount.admins).toContainEqual(admin3.publicKey);
    });

    test("Non-boss cannot remove admin - should fail", async () => {
        // when & then
        await expect(
            program.methods.removeAdmin(admin1.publicKey).signers([nonBoss]).rpc()
        ).rejects.toThrow();
    });

    test("Cannot remove admin that doesn't exist", async () => {
        // given
        const nonExistentAdmin = testHelper.createUserAccount();

        // when & then
        await expect(
            program.methods.removeAdmin(nonExistentAdmin.publicKey).rpc()
        ).rejects.toThrow();
    });

    test("Can remove all admins one by one", async () => {
        // given
        const initialAdminState = await program.account.adminState.fetch(adminStatePda);
        expect(initialAdminState.admins).toHaveLength(3);

        // when - remove first admin
        await program.methods.removeAdmin(admin1.publicKey).rpc();

        // then
        let adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).toHaveLength(2);
        expect(adminStateAccount.admins).not.toContainEqual(admin1.publicKey);

        // when - remove second admin
        await program.methods.removeAdmin(admin2.publicKey).rpc();

        // then
        adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).toHaveLength(1);
        expect(adminStateAccount.admins).not.toContainEqual(admin2.publicKey);

        // when - remove third admin
        await program.methods.removeAdmin(admin3.publicKey).rpc();

        // then
        adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).toHaveLength(0);
        expect(adminStateAccount.admins).not.toContainEqual(admin3.publicKey);
    });

    test("Can remove and re-add the same admin", async () => {
        // given - remove admin1
        await program.methods.removeAdmin(admin1.publicKey).rpc();

        let adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).not.toContainEqual(admin1.publicKey);
        expect(adminStateAccount.admins).toHaveLength(2);

        // when - re-add admin1
        await program.methods.addAdmin(admin1.publicKey).rpc();

        // then
        adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).toContainEqual(admin1.publicKey);
        expect(adminStateAccount.admins).toHaveLength(3);
    });

    test("Boss can remove themselves if they are in the admin list", async () => {
        // given - add boss as admin
        await program.methods.addAdmin(boss).rpc();

        let adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).toContainEqual(boss);
        expect(adminStateAccount.admins).toHaveLength(4);

        // when - boss removes themselves
        await program.methods.removeAdmin(boss).rpc();

        // then
        adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).not.toContainEqual(boss);
        expect(adminStateAccount.admins).toHaveLength(3);
    });

    test("Cannot remove the same admin twice", async () => {
        // given - remove admin1 first time
        await program.methods.removeAdmin(admin1.publicKey).rpc();

        // when & then - try to remove the same admin again
        await expect(
            program.methods.removeAdmin(admin1.publicKey).rpc()
        ).rejects.toThrow();
    });

    test("Removing admin preserves order of remaining admins", async () => {
        // given - verify initial order
        const initialAdminState = await program.account.adminState.fetch(adminStatePda);
        expect(initialAdminState.admins[0]).toEqual(admin1.publicKey);
        expect(initialAdminState.admins[1]).toEqual(admin2.publicKey);
        expect(initialAdminState.admins[2]).toEqual(admin3.publicKey);

        // when - remove middle admin
        await program.methods.removeAdmin(admin2.publicKey).rpc();

        // then - verify remaining admins (swap_remove moves last element to removed position)
        const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
        expect(adminStateAccount.admins).toHaveLength(2);
        expect(adminStateAccount.admins).toContainEqual(admin1.publicKey);
        expect(adminStateAccount.admins).toContainEqual(admin3.publicKey);
        expect(adminStateAccount.admins).not.toContainEqual(admin2.publicKey);
    });
});