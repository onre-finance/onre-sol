import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

describe("Vault Operations", () => {
    let testHelper: TestHelper;
    let program: Program<Onreapp>;
    
    let boss: PublicKey;
    let buyOfferVaultAuthorityPda: PublicKey;
    let singleRedemptionVaultAuthorityPda: PublicKey;
    let dualRedemptionVaultAuthorityPda: PublicKey;

    beforeEach(async () => {
        const programInfo: AddedProgram = {
            programId: ONREAPP_PROGRAM_ID,
            name: "onreapp",
        };

        const workspace = process.cwd();
        const context = await startAnchor(workspace, [programInfo], []);

        const provider = new BankrunProvider(context);
        program = new Program<Onreapp>(idl, provider);

        testHelper = new TestHelper(context, program);
        boss = provider.wallet.publicKey;
        
        // Get vault authority PDAs first
        [buyOfferVaultAuthorityPda] = PublicKey.findProgramAddressSync([Buffer.from('buy_offer_vault_authority')], ONREAPP_PROGRAM_ID);
        [singleRedemptionVaultAuthorityPda] = PublicKey.findProgramAddressSync([Buffer.from('single_redemption_vault_auth')], ONREAPP_PROGRAM_ID);
        [dualRedemptionVaultAuthorityPda] = PublicKey.findProgramAddressSync([Buffer.from('dual_redemption_vault_auth')], ONREAPP_PROGRAM_ID);
        
        // Initialize program and vault authorities
        await program.methods.initialize().accounts({ boss }).rpc();
        await program.methods.initializeVaultAuthority().accounts({ 
            state: testHelper.statePda,
            buyOfferVaultAuthority: buyOfferVaultAuthorityPda,
            singleRedemptionVaultAuthority: singleRedemptionVaultAuthorityPda,
            dualRedemptionVaultAuthority: dualRedemptionVaultAuthorityPda,
            boss
        }).rpc();
    });

    describe("Buy Offer Vault Operations", () => {
        test("Deposit tokens to buy offer vault should succeed", async () => {
            // given
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, buyOfferVaultAuthorityPda, true);
            const depositAmount = new BN(100_000e9);

            // when
            await program.methods.buyOfferVaultDeposit(depositAmount).accounts({
                tokenMint: testTokenMint,
                state: testHelper.statePda,
            }).rpc();

            // then
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(depositAmount.toString()));
            const expectedBossBalance = BigInt(1_000_000e9 - 100_000e9);
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
        });

        test("Non-boss cannot deposit to buy offer vault", async () => {
            // given
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const notBoss = testHelper.createUserAccount();
            const depositAmount = new BN(10_000e9);

            // when & then
            await expect(
                program.methods.buyOfferVaultDeposit(depositAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                    boss: notBoss.publicKey,
                }).signers([notBoss]).rpc()
            ).rejects.toThrow();
        });
    });

    describe("Single Redemption Vault Operations", () => {
        test("Deposit tokens to single redemption vault should succeed", async () => {
            // given
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, singleRedemptionVaultAuthorityPda, true);
            const depositAmount = new BN(200_000e9);

            // when
            await program.methods.singleRedemptionVaultDeposit(depositAmount).accounts({
                tokenMint: testTokenMint,
                state: testHelper.statePda,
            }).rpc();

            // then
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(depositAmount.toString()));
            const expectedBossBalance = BigInt(1_000_000e9 - 200_000e9);
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
        });

        test("Non-boss cannot deposit to single redemption vault", async () => {
            // given
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const notBoss = testHelper.createUserAccount();
            const depositAmount = new BN(10_000e9);

            // when & then
            await expect(
                program.methods.singleRedemptionVaultDeposit(depositAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                    boss: notBoss.publicKey,
                }).signers([notBoss]).rpc()
            ).rejects.toThrow();
        });
    });

    describe("Dual Redemption Vault Operations", () => {
        test("Deposit tokens to dual redemption vault should succeed", async () => {
            // given
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, dualRedemptionVaultAuthorityPda, true);
            const depositAmount = new BN(300_000e9);

            // when
            await program.methods.dualRedemptionVaultDeposit(depositAmount).accounts({
                tokenMint: testTokenMint,
                state: testHelper.statePda,
            }).rpc();

            // then
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(depositAmount.toString()));
            const expectedBossBalance = BigInt(1_000_000e9 - 300_000e9);
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
        });

        test("Non-boss cannot deposit to dual redemption vault", async () => {
            // given
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const notBoss = testHelper.createUserAccount();
            const depositAmount = new BN(10_000e9);

            // when & then
            await expect(
                program.methods.dualRedemptionVaultDeposit(depositAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                    boss: notBoss.publicKey,
                }).signers([notBoss]).rpc()
            ).rejects.toThrow();
        });
    });

    describe("Multiple Vault Types", () => {
        test("Can deposit to all three vault types with same token", async () => {
            // given
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            
            const buyOfferVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, buyOfferVaultAuthorityPda, true);
            const singleRedemptionVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, singleRedemptionVaultAuthorityPda, true);
            const dualRedemptionVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, dualRedemptionVaultAuthorityPda, true);
            
            const depositAmount = new BN(100_000e9);

            // when - deposit to all three vaults
            await program.methods.buyOfferVaultDeposit(depositAmount).accounts({
                tokenMint: testTokenMint,
                state: testHelper.statePda,
            }).rpc();

            await program.methods.singleRedemptionVaultDeposit(depositAmount).accounts({
                tokenMint: testTokenMint,
                state: testHelper.statePda,
            }).rpc();

            await program.methods.dualRedemptionVaultDeposit(depositAmount).accounts({
                tokenMint: testTokenMint,
                state: testHelper.statePda,
            }).rpc();

            // then - verify all vault accounts have correct balances
            await testHelper.expectTokenAccountAmountToBe(buyOfferVaultTokenAccount, BigInt(depositAmount.toString()));
            await testHelper.expectTokenAccountAmountToBe(singleRedemptionVaultTokenAccount, BigInt(depositAmount.toString()));
            await testHelper.expectTokenAccountAmountToBe(dualRedemptionVaultTokenAccount, BigInt(depositAmount.toString()));
            
            // Verify boss balance decreased by total amount deposited
            const expectedBossBalance = BigInt(1_000_000e9 - (3 * 100_000e9));
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
        });

        test("Vaults are isolated - deposits don't affect other vault types", async () => {
            // given
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            
            const buyOfferVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, buyOfferVaultAuthorityPda, true);
            const singleRedemptionVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, singleRedemptionVaultAuthorityPda, true);
            
            // when - only deposit to buy offer vault
            await program.methods.buyOfferVaultDeposit(new BN(100_000e9)).accounts({
                tokenMint: testTokenMint,
                state: testHelper.statePda,
            }).rpc();

            // then - only buy offer vault should have tokens
            await testHelper.expectTokenAccountAmountToBe(buyOfferVaultTokenAccount, BigInt(100_000e9));
            
            // Other vault accounts shouldn't exist yet (or have 0 balance if they were created)
            try {
                const balance = await testHelper.getTokenAccountBalance(singleRedemptionVaultTokenAccount);
                expect(balance).toBe(BigInt(0));
            } catch {
                // Account doesn't exist, which is expected
            }
        });
    });

    describe("Vault Withdrawal Operations", () => {
        describe("Buy Offer Vault Withdrawals", () => {
            test("Withdraw tokens from buy offer vault should succeed", async () => {
                // given - deposit tokens first
                const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
                const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
                const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, buyOfferVaultAuthorityPda, true);
                const depositAmount = new BN(100_000e9);

                await program.methods.buyOfferVaultDeposit(depositAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                // when - withdraw some tokens
                const withdrawAmount = new BN(50_000e9);
                await program.methods.buyOfferVaultWithdraw(withdrawAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                // then - verify balances
                await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(50_000e9)); // 100k - 50k
                await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, BigInt(950_000e9)); // original 900k + 50k withdrawn
            });

            test("Non-boss cannot withdraw from buy offer vault", async () => {
                // given
                const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
                const notBoss = testHelper.createUserAccount();
                const withdrawAmount = new BN(10_000e9);

                // when & then
                await expect(
                    program.methods.buyOfferVaultWithdraw(withdrawAmount).accounts({
                        tokenMint: testTokenMint,
                        state: testHelper.statePda,
                        boss: notBoss.publicKey,
                    }).signers([notBoss]).rpc()
                ).rejects.toThrow();
            });
        });

        describe("Single Redemption Vault Withdrawals", () => {
            test("Withdraw tokens from single redemption vault should succeed", async () => {
                // given - deposit tokens first
                const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
                const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
                const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, singleRedemptionVaultAuthorityPda, true);
                const depositAmount = new BN(200_000e9);

                await program.methods.singleRedemptionVaultDeposit(depositAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                // when - withdraw some tokens
                const withdrawAmount = new BN(75_000e9);
                await program.methods.singleRedemptionVaultWithdraw(withdrawAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                // then - verify balances
                await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(125_000e9)); // 200k - 75k
                await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, BigInt(875_000e9)); // original 800k + 75k withdrawn
            });

            test("Non-boss cannot withdraw from single redemption vault", async () => {
                // given
                const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
                const notBoss = testHelper.createUserAccount();
                const withdrawAmount = new BN(10_000e9);

                // when & then
                await expect(
                    program.methods.singleRedemptionVaultWithdraw(withdrawAmount).accounts({
                        tokenMint: testTokenMint,
                        state: testHelper.statePda,
                        boss: notBoss.publicKey,
                    }).signers([notBoss]).rpc()
                ).rejects.toThrow();
            });
        });

        describe("Dual Redemption Vault Withdrawals", () => {
            test("Withdraw tokens from dual redemption vault should succeed", async () => {
                // given - deposit tokens first
                const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
                const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
                const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, dualRedemptionVaultAuthorityPda, true);
                const depositAmount = new BN(300_000e9);

                await program.methods.dualRedemptionVaultDeposit(depositAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                // when - withdraw some tokens
                const withdrawAmount = new BN(100_000e9);
                await program.methods.dualRedemptionVaultWithdraw(withdrawAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                // then - verify balances
                await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(200_000e9)); // 300k - 100k
                await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, BigInt(800_000e9)); // original 700k + 100k withdrawn
            });

            test("Non-boss cannot withdraw from dual redemption vault", async () => {
                // given
                const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
                const notBoss = testHelper.createUserAccount();
                const withdrawAmount = new BN(10_000e9);

                // when & then
                await expect(
                    program.methods.dualRedemptionVaultWithdraw(withdrawAmount).accounts({
                        tokenMint: testTokenMint,
                        state: testHelper.statePda,
                        boss: notBoss.publicKey,
                    }).signers([notBoss]).rpc()
                ).rejects.toThrow();
            });
        });

        describe("Combined Deposit and Withdraw Operations", () => {
            test("Can deposit to all vaults and withdraw from each independently", async () => {
                // given
                const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
                const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
                
                const buyOfferVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, buyOfferVaultAuthorityPda, true);
                const singleRedemptionVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, singleRedemptionVaultAuthorityPda, true);
                const dualRedemptionVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, dualRedemptionVaultAuthorityPda, true);
                
                const depositAmount = new BN(100_000e9);

                // when - deposit to all three vaults
                await program.methods.buyOfferVaultDeposit(depositAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                await program.methods.singleRedemptionVaultDeposit(depositAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                await program.methods.dualRedemptionVaultDeposit(depositAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                // then - withdraw different amounts from each vault
                const withdrawAmount1 = new BN(30_000e9);
                const withdrawAmount2 = new BN(50_000e9);
                const withdrawAmount3 = new BN(70_000e9);

                await program.methods.buyOfferVaultWithdraw(withdrawAmount1).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                await program.methods.singleRedemptionVaultWithdraw(withdrawAmount2).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                await program.methods.dualRedemptionVaultWithdraw(withdrawAmount3).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                // verify final vault balances
                await testHelper.expectTokenAccountAmountToBe(buyOfferVaultTokenAccount, BigInt(70_000e9)); // 100k - 30k
                await testHelper.expectTokenAccountAmountToBe(singleRedemptionVaultTokenAccount, BigInt(50_000e9)); // 100k - 50k
                await testHelper.expectTokenAccountAmountToBe(dualRedemptionVaultTokenAccount, BigInt(30_000e9)); // 100k - 70k
                
                // Verify boss balance (original - total deposited + total withdrawn)
                // 1M - 300k + 150k = 850k
                const expectedBossBalance = BigInt(1_000_000e9 - 3 * 100_000e9 + 30_000e9 + 50_000e9 + 70_000e9);
                await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
            });

            test("Cannot withdraw more than vault balance", async () => {
                // given - small deposit
                const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
                testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
                const depositAmount = new BN(10_000e9);

                await program.methods.buyOfferVaultDeposit(depositAmount).accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                }).rpc();

                // when & then - attempt to withdraw more than deposited
                const withdrawAmount = new BN(20_000e9);
                await expect(
                    program.methods.buyOfferVaultWithdraw(withdrawAmount).accounts({
                        tokenMint: testTokenMint,
                        state: testHelper.statePda,
                    }).rpc()
                ).rejects.toThrow();
            });
        });
    });
});