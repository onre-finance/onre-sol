import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";
import { getAssociatedTokenAddressSync, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";

describe("Vault Operations", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let boss: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);
        boss = testHelper.getBoss();

        // Initialize program and vault authorities
        await program.initialize({ onycMint: testHelper.createMint(9) });
    });

    test("Vault is initialized correctly", async () => {
        // Verify all vault authorities are initialized correctly
        const offerVaultAuthority = await program.program.provider.connection.getAccountInfo(program.pdas.offerVaultAuthorityPda);

        expect(offerVaultAuthority).toBeDefined();
    });

    describe("Offer Vault Operations", () => {
        test("Deposit tokens to offer vault should succeed", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.offerVaultAuthorityPda, true);
            const depositAmount = 100_000e9;

            // when
            await program.offerVaultDeposit({
                amount: depositAmount,
                tokenMint: testTokenMint
            });

            // then
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(depositAmount));
            const expectedBossBalance = BigInt(1_000_000e9 - 100_000e9);
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
        });

        test("Non-boss cannot deposit to offer vault", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));

            const notBoss = testHelper.createUserAccount();
            testHelper.createTokenAccount(testTokenMint, notBoss.publicKey, BigInt(1_000_000e9));
            const depositAmount = 10_000e9;

            // when & then
            await expect(
                program.offerVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint,
                    signer: notBoss
                })
            ).rejects.toThrow("unknown signer");
        });
    });

    describe("Vault Withdrawal Operations", () => {
        describe("Offer Vault Withdrawals", () => {
            test("Withdraw tokens from offer vault should succeed", async () => {
                // given - deposit tokens first
                const testTokenMint = testHelper.createMint(9);
                const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
                const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.offerVaultAuthorityPda, true);
                const depositAmount = 100_000e9;

                await program.offerVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint
                });

                // when - withdraw some tokens
                const withdrawAmount = 50_000e9;
                await program.offerVaultWithdraw({
                    amount: withdrawAmount,
                    tokenMint: testTokenMint
                });

                // then - verify balances
                await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(50_000e9)); // 100k - 50k
                await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, BigInt(950_000e9)); // original 900k + 50k withdrawn
            });

            test("Non-boss cannot withdraw from offer vault", async () => {
                // given
                const testTokenMint = testHelper.createMint(9);
                const notBoss = testHelper.createUserAccount();
                const withdrawAmount = 10_000e9;

                // when & then
                await expect(
                    program.offerVaultWithdraw({
                        amount: withdrawAmount,
                        tokenMint: testTokenMint,
                        signer: notBoss
                    })
                ).rejects.toThrow("unknown signer");
            });

            test("Withdraw Token2022 tokens from offer vault should succeed", async () => {
                // given - create Token2022 mint and deposit tokens
                const testTokenMint = testHelper.createMint2022(9);
                const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9), false, TOKEN_2022_PROGRAM_ID);
                const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.offerVaultAuthorityPda, true, TOKEN_2022_PROGRAM_ID);
                const depositAmount = 100_000e9;

                await program.offerVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint,
                    tokenProgram: TOKEN_2022_PROGRAM_ID
                });

                // when - withdraw Token2022 tokens
                const withdrawAmount = 50_000e9;
                await program.offerVaultWithdraw({
                    amount: withdrawAmount,
                    tokenMint: testTokenMint,
                    tokenProgram: TOKEN_2022_PROGRAM_ID
                });

                // then - verify balances
                await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(50_000e9));
                await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, BigInt(950_000e9));
            });

            test("Withdraw with wrong token_program should fail", async () => {
                // given - deposit tokens first
                const testTokenMint = testHelper.createMint(9);
                const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
                const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.offerVaultAuthorityPda, true);
                const depositAmount = 100_000e9;

                await program.offerVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint
                });

                // when - withdraw some tokens
                const withdrawAmount = 50_000e9;
                await expect(program.offerVaultWithdraw({
                    amount: withdrawAmount,
                    tokenMint: testTokenMint,
                    tokenProgram: TOKEN_2022_PROGRAM_ID
                })).rejects.toThrow();
            });
        });
    });

    describe("Redemption Vault Operations", () => {
        test("Deposit tokens to redemption vault should succeed", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.redemptionVaultAuthorityPda, true);
            const depositAmount = 100_000e9;

            // when
            await program.redemptionVaultDeposit({
                amount: depositAmount,
                tokenMint: testTokenMint
            });

            // then
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(depositAmount));
            const expectedBossBalance = BigInt(1_000_000e9 - 100_000e9);
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
        });

        test("Non-boss cannot deposit to redemption vault", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));

            const notBoss = testHelper.createUserAccount();
            testHelper.createTokenAccount(testTokenMint, notBoss.publicKey, BigInt(1_000_000e9));
            const depositAmount = 10_000e9;

            // when & then
            await expect(
                program.redemptionVaultDeposit({
                    amount: depositAmount,
                    tokenMint: testTokenMint,
                    signer: notBoss
                })
            ).rejects.toThrow("unknown signer");
        });
    });

    describe("Redemption Vault Withdrawal Operations", () => {
        test("Withdraw tokens from redemption vault should succeed", async () => {
            // given - deposit tokens first
            const testTokenMint = testHelper.createMint(9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.redemptionVaultAuthorityPda, true);
            const depositAmount = 100_000e9;

            await program.redemptionVaultDeposit({
                amount: depositAmount,
                tokenMint: testTokenMint
            });

            // when - withdraw some tokens
            const withdrawAmount = 50_000e9;
            await program.redemptionVaultWithdraw({
                amount: withdrawAmount,
                tokenMint: testTokenMint
            });

            // then - verify balances
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(50_000e9)); // 100k - 50k
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, BigInt(950_000e9)); // original 900k + 50k withdrawn
        });

        test("Non-boss cannot withdraw from redemption vault", async () => {
            // given
            const testTokenMint = testHelper.createMint(9);
            const notBoss = testHelper.createUserAccount();
            const withdrawAmount = 10_000e9;

            // when & then
            await expect(
                program.redemptionVaultWithdraw({
                    amount: withdrawAmount,
                    tokenMint: testTokenMint,
                    signer: notBoss
                })
            ).rejects.toThrow("unknown signer");
        });

        test("Withdraw Token2022 tokens from redemption vault should succeed", async () => {
            // given - create Token2022 mint and deposit tokens
            const testTokenMint = testHelper.createMint2022(9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9), false, TOKEN_2022_PROGRAM_ID);
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.redemptionVaultAuthorityPda, true, TOKEN_2022_PROGRAM_ID);
            const depositAmount = 100_000e9;

            await program.redemptionVaultDeposit({
                amount: depositAmount,
                tokenMint: testTokenMint,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            // when - withdraw Token2022 tokens
            const withdrawAmount = 50_000e9;
            await program.redemptionVaultWithdraw({
                amount: withdrawAmount,
                tokenMint: testTokenMint,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            });

            // then - verify balances
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(50_000e9));
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, BigInt(950_000e9));
        });

        test("Withdraw with wrong token_program should fail", async () => {
            // given - deposit tokens first
            const testTokenMint = testHelper.createMint(9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, program.pdas.redemptionVaultAuthorityPda, true);
            const depositAmount = 100_000e9;

            await program.redemptionVaultDeposit({
                amount: depositAmount,
                tokenMint: testTokenMint
            });

            // when - withdraw some tokens
            const withdrawAmount = 50_000e9;
            await expect(program.redemptionVaultWithdraw({
                amount: withdrawAmount,
                tokenMint: testTokenMint,
                tokenProgram: TOKEN_2022_PROGRAM_ID
            })).rejects.toThrow();
        });
    });
});