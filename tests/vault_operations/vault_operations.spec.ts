import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

describe("Vault Operations", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let boss: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);
        boss = testHelper.getBoss();

        // Initialize program and vault authorities
        await program.initialize();
        await program.initializeVaultAuthority();
    });

    test("Vaults are initialized correctly", async () => {
        // Verify all vault authorities are initialized correctly
        const buyOfferVaultAuthority = await program.program.account.buyOfferVaultAuthority.fetch(program.pdas.buyOfferVaultAuthorityPda);
        const singleRedemptionVaultAuthority = await program.program.account.singleRedemptionVaultAuthority.fetch(program.pdas.singleRedemptionVaultAuthorityPda);
        const dualRedemptionVaultAuthority = await program.program.account.dualRedemptionVaultAuthority.fetch(program.pdas.dualRedemptionVaultAuthorityPda);

        expect(buyOfferVaultAuthority).toBeDefined();
        expect(singleRedemptionVaultAuthority).toBeDefined();
        expect(dualRedemptionVaultAuthority).toBeDefined();
    });

    describe("Buy Offer Vault Operations", () => {
        test("Deposit tokens to buy offer vault should succeed", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.buyOfferVaultAuthorityPda, true);
            const depositAmount = 100_000e9;

            // when
            await program.buyOfferVaultDeposit({
                amount: depositAmount,
                tokenMint: testTokenMint
            });

            // then
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(depositAmount));
            const expectedBossBalance = BigInt(1_000_000e9 - 100_000e9);
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
        });

        test("Non-boss cannot deposit to buy offer vault", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));

            const notBoss = testHelper.createUserAccount();
            testHelper.createTokenAccount(testTokenMint, notBoss.publicKey, BigInt(1_000_000e9));
            const depositAmount = 10_000e9;

            // when & then
            await expect(
                program.buyOfferVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint,
                    signer: notBoss
                })
            ).rejects.toThrow("unknown signer");
        });
    });

    describe("Single Redemption Vault Operations", () => {
        test("Deposit tokens to single redemption vault should succeed", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.singleRedemptionVaultAuthorityPda, true);
            const depositAmount = 200_000e9;

            // when
            await program.singleRedemptionVaultDeposit({
                amount: depositAmount,
                tokenMint: testTokenMint
            });

            // then
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(depositAmount));
            const expectedBossBalance = BigInt(1_000_000e9 - 200_000e9);
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
        });

        test("Non-boss cannot deposit to single redemption vault", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const notBoss = testHelper.createUserAccount();
            testHelper.createTokenAccount(testTokenMint, notBoss.publicKey, BigInt(1_000_000e9));
            const depositAmount = 10_000e9;

            // when & then
            await expect(
                program.singleRedemptionVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint,
                    signer: notBoss
                })
            ).rejects.toThrow("unknown signer");
        });
    });

    describe("Dual Redemption Vault Operations", () => {
        test("Deposit tokens to dual redemption vault should succeed", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.dualRedemptionVaultAuthorityPda, true);
            const depositAmount = 300_000e9;

            // when
            await program.dualRedemptionVaultDeposit({
                amount: depositAmount,
                tokenMint: testTokenMint
            });

            // then
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(depositAmount));
            const expectedBossBalance = BigInt(1_000_000e9 - 300_000e9);
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
        });

        test("Non-boss cannot deposit to dual redemption vault", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const notBoss = testHelper.createUserAccount();
            testHelper.createTokenAccount(testTokenMint, notBoss.publicKey, BigInt(1_000_000e9));
            const depositAmount = 10_000e9;

            // when & then
            await expect(
                program.dualRedemptionVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint,
                    signer: notBoss
                })
            ).rejects.toThrow("unknown signer");
        });
    });

    describe("Multiple Vault Types", () => {
        test("Can deposit to all three vault types with same token", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));

            const buyOfferVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.buyOfferVaultAuthorityPda, true);
            const singleRedemptionVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.singleRedemptionVaultAuthorityPda, true);
            const dualRedemptionVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.dualRedemptionVaultAuthorityPda, true);

            const depositAmount = 100_000e9;

            // when - deposit to all three vaults
            await program.buyOfferVaultDeposit({
                amount: depositAmount,
                tokenMint: testTokenMint
            });

            await program.singleRedemptionVaultDeposit({
                amount: depositAmount,
                tokenMint: testTokenMint
            });

            await program.dualRedemptionVaultDeposit({
                amount: depositAmount,
                tokenMint: testTokenMint
            });

            // then - verify all vault accounts have correct balances
            await testHelper.expectTokenAccountAmountToBe(buyOfferVaultTokenAccount, BigInt(depositAmount));
            await testHelper.expectTokenAccountAmountToBe(singleRedemptionVaultTokenAccount, BigInt(depositAmount));
            await testHelper.expectTokenAccountAmountToBe(dualRedemptionVaultTokenAccount, BigInt(depositAmount));

            // Verify boss balance decreased by total amount deposited
            const expectedBossBalance = BigInt(1_000_000e9 - (3 * 100_000e9));
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
        });

        test("Vaults are isolated - deposits don't affect other vault types", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));

            const buyOfferVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.buyOfferVaultAuthorityPda, true);
            const singleRedemptionVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.singleRedemptionVaultAuthorityPda, true);
            const dualRedemptionVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.dualRedemptionVaultAuthorityPda, true);

            // when - only deposit to buy offer vault
            await program.buyOfferVaultDeposit({
                amount: 100_000e9,
                tokenMint: testTokenMint
            });

            // then - only buy offer vault should have tokens
            await testHelper.expectTokenAccountAmountToBe(buyOfferVaultTokenAccount, BigInt(100_000e9));

            // Other vault accounts shouldn't exist yet (or have 0 balance if they were created)
            await expect(testHelper.getTokenAccountBalance(singleRedemptionVaultTokenAccount)).rejects.toThrow("Token account not found");
            await expect(testHelper.getTokenAccountBalance(dualRedemptionVaultTokenAccount)).rejects.toThrow("Token account not found");
        });
    });

    describe("Vault Withdrawal Operations", () => {
        describe("Buy Offer Vault Withdrawals", () => {
            test("Withdraw tokens from buy offer vault should succeed", async () => {
                // given - deposit tokens first
                const testTokenMint = testHelper.createMint(9);
                const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
                const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.buyOfferVaultAuthorityPda, true);
                const depositAmount = 100_000e9;

                await program.buyOfferVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint
                });

                // when - withdraw some tokens
                const withdrawAmount = 50_000e9;
                await program.buyOfferVaultWithdraw({
                    amount: withdrawAmount,
                    tokenMint: testTokenMint
                });

                // then - verify balances
                await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(50_000e9)); // 100k - 50k
                await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, BigInt(950_000e9)); // original 900k + 50k withdrawn
            });

            test("Non-boss cannot withdraw from buy offer vault", async () => {
                // given
                const testTokenMint = testHelper.createMint(9);
                const notBoss = testHelper.createUserAccount();
                const withdrawAmount = 10_000e9;

                // when & then
                await expect(
                    program.buyOfferVaultWithdraw({
                        amount: withdrawAmount,
                        tokenMint: testTokenMint,
                        signer: notBoss
                    })
                ).rejects.toThrow("unknown signer");
            });
        });

        describe("Single Redemption Vault Withdrawals", () => {
            test("Withdraw tokens from single redemption vault should succeed", async () => {
                // given - deposit tokens first
                const testTokenMint = testHelper.createMint(9);
                const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
                const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.singleRedemptionVaultAuthorityPda, true);
                const depositAmount = 200_000e9;

                await program.singleRedemptionVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint
                });

                // when - withdraw some tokens
                const withdrawAmount = 75_000e9;
                await program.singleRedemptionVaultWithdraw({
                    amount: withdrawAmount,
                    tokenMint: testTokenMint
                });

                // then - verify balances
                await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(125_000e9)); // 200k - 75k
                await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, BigInt(875_000e9)); // original 800k + 75k withdrawn
            });

            test("Non-boss cannot withdraw from single redemption vault", async () => {
                // given
                const testTokenMint = testHelper.createMint(9);
                const notBoss = testHelper.createUserAccount();
                const withdrawAmount = 10_000e9;

                // when & then
                await expect(
                    program.singleRedemptionVaultWithdraw({
                        amount: withdrawAmount,
                        tokenMint: testTokenMint,
                        signer: notBoss
                    })
                ).rejects.toThrow("unknown signer");
            });
        });

        describe("Dual Redemption Vault Withdrawals", () => {
            test("Withdraw tokens from dual redemption vault should succeed", async () => {
                // given - deposit tokens first
                const testTokenMint = testHelper.createMint(9);
                const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
                const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.dualRedemptionVaultAuthorityPda, true);
                const depositAmount = 300_000e9;

                await program.dualRedemptionVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint
                });

                // when - withdraw some tokens
                const withdrawAmount = 100_000e9;
                await program.dualRedemptionVaultWithdraw({
                    amount: withdrawAmount,
                    tokenMint: testTokenMint
                });

                // then - verify balances
                await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(200_000e9)); // 300k - 100k
                await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, BigInt(800_000e9)); // original 700k + 100k withdrawn
            });

            test("Non-boss cannot withdraw from dual redemption vault", async () => {
                // given
                const testTokenMint = testHelper.createMint(9);
                const notBoss = testHelper.createUserAccount();
                const withdrawAmount = 10_000e9;

                // when & then
                await expect(
                    program.dualRedemptionVaultWithdraw({
                        amount: withdrawAmount,
                        tokenMint: testTokenMint,
                        signer: notBoss
                    })
                ).rejects.toThrow("unknown signer");
            });
        });

        describe("Combined Deposit and Withdraw Operations", () => {
            test("Can deposit to all vaults and withdraw from each independently", async () => {
                // given
                const testTokenMint = testHelper.createMint(9);
                const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));

                const buyOfferVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.buyOfferVaultAuthorityPda, true);
                const singleRedemptionVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.singleRedemptionVaultAuthorityPda, true);
                const dualRedemptionVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.dualRedemptionVaultAuthorityPda, true);

                const depositAmount = 100_000e9;

                // when - deposit to all three vaults
                await program.buyOfferVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint
                });

                await program.singleRedemptionVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint
                });

                await program.dualRedemptionVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint
                });

                // then - withdraw different amounts from each vault
                const withdrawAmount1 = 30_000e9;
                const withdrawAmount2 = 50_000e9;
                const withdrawAmount3 = 70_000e9;

                await program.buyOfferVaultWithdraw({
                    amount: withdrawAmount1,
                    tokenMint: testTokenMint
                });

                await program.singleRedemptionVaultWithdraw({
                    amount: withdrawAmount2,
                    tokenMint: testTokenMint
                });

                await program.dualRedemptionVaultWithdraw({
                    amount: withdrawAmount3,
                    tokenMint: testTokenMint
                });

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
                const testTokenMint = testHelper.createMint(9);
                testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
                const depositAmount = 10_000e9;

                await program.buyOfferVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint
                });

                // when & then - attempt to withdraw more than deposited
                const withdrawAmount = 20_000e9;
                await expect(
                    program.buyOfferVaultWithdraw({
                        amount: withdrawAmount,
                        tokenMint: testTokenMint
                    })
                ).rejects.toThrow();
            });
        });
    });
});