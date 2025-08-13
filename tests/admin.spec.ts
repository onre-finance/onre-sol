import { describe, test, beforeAll } from "@jest/globals";
import { startAnchor } from "solana-bankrun";
import { BankrunProvider } from "anchor-bankrun";
import { Keypair, PublicKey, VersionedTransaction, TransactionMessage } from "@solana/web3.js";
import { Program } from "@coral-xyz/anchor";
import { OnreApp } from "../target/types/onre_app";
import { ONREAPP_PROGRAM_ID, TestHelper } from "./test_helper";

describe("Admin Management Tests", () => {
    let testHelper: TestHelper;
    let provider: BankrunProvider;
    let program: Program<OnreApp>;
    let boss: PublicKey;
    let admin1: Keypair;
    let admin2: Keypair;
    let admin3: Keypair;
    let nonAdmin: Keypair;
    let adminStatePda: PublicKey;

    beforeAll(async () => {
        // Initialize keypairs
        admin1 = Keypair.generate();
        admin2 = Keypair.generate();
        admin3 = Keypair.generate();
        nonAdmin = Keypair.generate();

        // Start Anchor test environment
        const context = await startAnchor("", [{ name: "onreapp", programId: ONREAPP_PROGRAM_ID }], []);
        provider = new BankrunProvider(context);
        program = new Program<OnreApp>(require("../target/idl/onre_app.json"), provider);
        testHelper = new TestHelper(context, program);

        // Derive admin state PDA
        [adminStatePda] = PublicKey.findProgramAddressSync([Buffer.from("admin_state")], ONREAPP_PROGRAM_ID);

        // Initialize the main state with boss
        boss = provider.wallet.publicKey;
        await program.methods.initialize().accounts({ boss }).rpc();

        // Initialize admin state
        await program.methods
            .initializeAdminState()
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Fund admin and nonAdmin accounts for transactions
        await testHelper.airdropLamports(admin1.publicKey, 1000000000);
        await testHelper.airdropLamports(admin2.publicKey, 1000000000);
        await testHelper.airdropLamports(admin3.publicKey, 1000000000);
        await testHelper.airdropLamports(nonAdmin.publicKey, 1000000000);
    });

    describe("Initialize Admin State", () => {
        test("should initialize admin state successfully", async () => {
            const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
            expect(adminStateAccount.admins.length).toBe(0);
        });
    });

    describe("Add Admin", () => {
        test("boss should be able to add an admin", async () => {
            await program.methods
                .addAdmin(admin1.publicKey)
                .accounts({
                    state: testHelper.statePda,
                    admin: boss,
                })

                .rpc();

            const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
            expect(adminStateAccount.admins.map(k => k.toBase58())).toContain(admin1.publicKey.toBase58());
            expect(adminStateAccount.admins.length).toBe(1);
        });

        test("existing admin should be able to add another admin", async () => {
            await program.methods
                .addAdmin(admin2.publicKey)
                .accounts({
                    state: testHelper.statePda,
                    admin: admin1.publicKey,
                })
                .signers([admin1])
                .rpc();

            const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
            expect(adminStateAccount.admins.map(k => k.toBase58())).toContain(admin2.publicKey.toBase58());
            expect(adminStateAccount.admins.length).toBe(2);
        });

        test("should fail when non-admin tries to add admin", async () => {
            try {
                await program.methods
                    .addAdmin(admin3.publicKey)
                    .accounts({
                        state: testHelper.statePda,
                        admin: nonAdmin.publicKey,
                    })
                    .signers([nonAdmin])
                    .rpc();
                expect(true).toBe(false); // Should not reach here
            } catch (error) {
                expect(error.toString()).toContain("CallerNotAdmin");
            }
        });

        test("should fail when trying to add duplicate admin", async () => {
            try {
                await program.methods
                    .addAdmin(admin1.publicKey)
                    .accounts({
                        state: testHelper.statePda,
                        admin: boss,
                    })

                    .rpc();
                expect(true).toBe(false); // Should not reach here
            } catch (error) {
                expect(error.toString()).toContain("AdminAlreadyExists");
            }
        });

        test("should fail when trying to add more than 20 admins", async () => {
            // Add admins up to the limit (we already have admin1 + admin2 = 2)
            const adminsToAdd = [];
            for (let i = 0; i < 18; i++) {
                adminsToAdd.push(Keypair.generate());
            }

            // Add 17 more admins to reach 20 total
            for (const admin of adminsToAdd) {
                await program.methods
                    .addAdmin(admin.publicKey)
                    .accounts({
                        state: testHelper.statePda,
                        admin: boss,
                    })

                    .rpc();
            }

            // Verify we have 20 admins
            const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
            expect(adminStateAccount.admins.length).toBe(20);

            // Try to add one more (should fail)
            const extraAdmin = Keypair.generate();
            try {
                await program.methods
                    .addAdmin(extraAdmin.publicKey)
                    .accounts({
                        state: testHelper.statePda,
                        admin: boss,
                    })

                    .rpc();
                expect(true).toBe(false); // Should not reach here
            } catch (error) {
                expect(error.toString()).toContain("MaxAdminsReached");
            }
        });
    });

    describe("Remove Admin", () => {
        test("boss should be able to remove an admin", async () => {
            const adminStateAccountBefore = await program.account.adminState.fetch(adminStatePda);
            const initialCount = adminStateAccountBefore.admins.length;

            await program.methods
                .removeAdmin(admin2.publicKey)
                .accounts({
                    state: testHelper.statePda,
                    admin: boss,
                })

                .rpc();

            const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
            expect(adminStateAccount.admins.map(k => k.toBase58())).not.toContain(admin2.publicKey.toBase58());
            expect(adminStateAccount.admins.length).toBe(initialCount - 1);
        });

        test("existing admin should be able to remove another admin", async () => {
            const adminStateAccountBefore = await program.account.adminState.fetch(adminStatePda);
            const initialCount = adminStateAccountBefore.admins.length;

            // First add admin3 so we can remove it
            await program.methods
                .addAdmin(admin3.publicKey)
                .accounts({
                    state: testHelper.statePda,
                    admin: boss,
                })

                .rpc();

            // Now remove admin3 using admin1
            await program.methods
                .removeAdmin(admin3.publicKey)
                .accounts({
                    state: testHelper.statePda,
                    admin: admin1.publicKey,
                })
                .signers([admin1])
                .rpc();

            const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
            expect(adminStateAccount.admins.map(k => k.toBase58())).not.toContain(admin3.publicKey.toBase58());
            expect(adminStateAccount.admins.length).toBe(initialCount);
        });

        test("should fail when non-admin tries to remove admin", async () => {
            try {
                await program.methods
                    .removeAdmin(admin1.publicKey)
                    .accounts({
                        state: testHelper.statePda,
                        admin: nonAdmin.publicKey,
                    })
                    .signers([nonAdmin])
                    .rpc();
                expect(true).toBe(false); // Should not reach here
            } catch (error) {
                expect(error.toString()).toContain("CallerNotAdmin");
            }
        });

        test("should fail when trying to remove non-existent admin", async () => {
            const nonExistentAdmin = Keypair.generate();
            try {
                await program.methods
                    .removeAdmin(nonExistentAdmin.publicKey)
                    .accounts({
                        state: testHelper.statePda,
                        admin: boss,
                    })

                    .rpc();
                expect(true).toBe(false); // Should not reach here
            } catch (error) {
                expect(error.toString()).toContain("AdminNotFound");
            }
        });
    });

    describe("Remove All Admins", () => {
        test("boss should be able to remove all admins", async () => {
            // First verify we have multiple admins
            const adminStateAccountBefore = await program.account.adminState.fetch(adminStatePda);
            expect(adminStateAccountBefore.admins.length).toBeGreaterThan(1);

            await program.methods
                .removeAllAdmins()
                .accounts({
                    state: testHelper.statePda,
                })

                .rpc();

            const adminStateAccount = await program.account.adminState.fetch(adminStatePda);
            expect(adminStateAccount.admins.length).toBe(0);
        });

        test("should fail when non-boss tries to remove all admins", async () => {
            // First add back some admins
            await program.methods
                .addAdmin(admin1.publicKey)
                .accounts({
                    state: testHelper.statePda,
                    admin: boss,
                })
                .rpc();

            // Create a wallet for nonAdmin
            const unauthorizedWallet = {
                publicKey: nonAdmin.publicKey,
                signTransaction: async (tx) => {
                    tx.sign([nonAdmin]);
                    return tx;
                },
            };

            const removeAllAdminsInstruction = await program.methods
                .removeAllAdmins()
                .accountsPartial({ 
                    state: testHelper.statePda, 
                    boss: nonAdmin.publicKey 
                })
                .instruction();

            const tx = new VersionedTransaction(
                new TransactionMessage({
                    payerKey: unauthorizedWallet.publicKey,
                    recentBlockhash: testHelper.context.lastBlockhash,
                    instructions: [removeAllAdminsInstruction],
                }).compileToLegacyMessage(),
            );
            const versionedTransaction = await unauthorizedWallet.signTransaction(tx);

            // Error Number: 2001. Error Message: A has one constraint was violated." 0x7d1 corresponds to 2001
            // This error indicates that the caller is not the boss
            await expect(testHelper.context.banksClient.processTransaction(versionedTransaction)).rejects.toThrow(/custom program error: 0x7d1/);
        });
    });

    describe("Integration Tests", () => {
        test("admin workflow: add multiple admins, remove some, remove all", async () => {
            // Start fresh
            await program.methods
                .removeAllAdmins()
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Add multiple admins
            await program.methods
                .addAdmin(admin1.publicKey)
                .accounts({
                    state: testHelper.statePda,
                    admin: boss,
                })
                .rpc();

            await program.methods
                .addAdmin(admin2.publicKey)
                .accounts({
                    state: testHelper.statePda,
                    admin: admin1.publicKey,
                })
                .signers([admin1])
                .rpc();

            await program.methods
                .addAdmin(admin3.publicKey)
                .accounts({
                    state: testHelper.statePda,
                    admin: admin2.publicKey,
                })
                .signers([admin2])
                .rpc();

            // Verify all admins are added
            let adminStateAccount = await program.account.adminState.fetch(adminStatePda);
            expect(adminStateAccount.admins.length).toBe(3);
            expect(adminStateAccount.admins.map(k => k.toBase58())).toContain(admin1.publicKey.toBase58());
            expect(adminStateAccount.admins.map(k => k.toBase58())).toContain(admin2.publicKey.toBase58());
            expect(adminStateAccount.admins.map(k => k.toBase58())).toContain(admin3.publicKey.toBase58());

            // Remove some admins
            await program.methods
                .removeAdmin(admin2.publicKey)
                .accounts({
                    state: testHelper.statePda,
                    admin: admin3.publicKey,
                })
                .signers([admin3])
                .rpc();

            adminStateAccount = await program.account.adminState.fetch(adminStatePda);
            expect(adminStateAccount.admins.length).toBe(2);
            expect(adminStateAccount.admins.map(k => k.toBase58())).not.toContain(admin2.publicKey.toBase58());

            // Remove all admins (boss only)
            await program.methods
                .removeAllAdmins()
                .accounts({
                    state: testHelper.statePda,
                })

                .rpc();

            adminStateAccount = await program.account.adminState.fetch(adminStatePda);
            expect(adminStateAccount.admins.length).toBe(0);
        });
    });
});
