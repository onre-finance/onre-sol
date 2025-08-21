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
    let vaultAuthorityPda: PublicKey;
    let tokenMint: PublicKey;
    let bossTokenAccount: PublicKey;
    let vaultTokenAccount: PublicKey;

    beforeEach(async () => {
        const programInfo: AddedProgram = {
            programId: ONREAPP_PROGRAM_ID,
            name: "onreapp",
        };

        const workspace = process.cwd();
        const context = await startAnchor(workspace, [programInfo], []);

        const provider = new BankrunProvider(context);
        program = new Program<Onreapp>(
            idl,
            provider,
        );

        testHelper = new TestHelper(context, program);
        boss = provider.wallet.publicKey;
        
        // Initialize program and vault authority
        await program.methods.initialize().accounts({ boss }).rpc();
        await program.methods.initializeVaultAuthority().accounts({ 
            state: testHelper.statePda 
        }).rpc();

        // Get vault authority PDA
        [vaultAuthorityPda] = PublicKey.findProgramAddressSync([Buffer.from('vault_authority')], ONREAPP_PROGRAM_ID);

        // Create token mint and boss token account with tokens
        tokenMint = testHelper.createMint(boss, BigInt(0), 9); // Create mint with 0 supply
        bossTokenAccount = testHelper.createTokenAccount(tokenMint, boss, BigInt(1_000_000e9)); // Create boss account with tokens
        vaultTokenAccount = getAssociatedTokenAddressSync(tokenMint, vaultAuthorityPda, true);
    });

    describe("Vault Deposit", () => {
        test("Deposit tokens should succeed and create vault token account", async () => {
            // given - create unique token for this test
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, vaultAuthorityPda, true);
            const depositAmount = new BN(100_000e9);

            // when
            await program.methods
                .vaultDeposit(depositAmount)
                .accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                })
                .rpc();

            // then - verify vault token account was created and has correct balance
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(depositAmount.toString()));

            // Verify boss token account balance decreased
            const expectedBossBalance = BigInt(1_000_000e9 - 100_000e9);
            await testHelper.expectTokenAccountAmountToBe(testBossTokenAccount, expectedBossBalance);
        });

        test("Deposit additional tokens should succeed", async () => {
            // given - create unique token and do initial deposit
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, vaultAuthorityPda, true);
            
            // First deposit to set up initial balance
            const initialDepositAmount = new BN(100_000e9);
            await program.methods
                .vaultDeposit(initialDepositAmount)
                .accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                })
                .rpc();
            
            const additionalAmount = new BN(50_000e9);
            
            // Get current balances after first deposit
            const initialVaultBalance = await testHelper.getTokenAccountBalance(testVaultTokenAccount);
            const initialBossBalance = await testHelper.getTokenAccountBalance(testBossTokenAccount);

            // when
            await program.methods
                .vaultDeposit(additionalAmount)
                .accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                })
                .rpc();

            // then
            const finalVaultBalance = await testHelper.getTokenAccountBalance(testVaultTokenAccount);
            const finalBossBalance = await testHelper.getTokenAccountBalance(testBossTokenAccount);

            expect(finalVaultBalance).toBe(initialVaultBalance + BigInt(additionalAmount.toString()));
            expect(finalBossBalance).toBe(initialBossBalance - BigInt(additionalAmount.toString()));
        });

        test("Deposit with different token mint should succeed", async () => {
            // given
            const newTokenMint = testHelper.createMint(boss, BigInt(0), 6);
            testHelper.createTokenAccount(newTokenMint, boss, BigInt(500_000e6));
            const newVaultTokenAccount = getAssociatedTokenAddressSync(newTokenMint, vaultAuthorityPda, true);
            const depositAmount = new BN(25_000e6);

            // when
            await program.methods
                .vaultDeposit(depositAmount)
                .accounts({
                    tokenMint: newTokenMint,
                    state: testHelper.statePda,
                })
                .rpc();

            // then
            await testHelper.expectTokenAccountAmountToBe(newVaultTokenAccount, BigInt(depositAmount.toString()));
        });

        test("Deposit should fail when not called by boss", async () => {
            // given
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const notBoss = testHelper.createUserAccount();
            const depositAmount = new BN(10_000e9);

            // when/then
            await expect(
                program.methods
                    .vaultDeposit(depositAmount)
                    .accountsPartial({
                        tokenMint: testTokenMint,
                        state: testHelper.statePda,
                        boss: notBoss.publicKey,
                    })
                    .signers([notBoss])
                    .rpc()
            ).rejects.toThrow(); // Should fail due to boss constraint
        });

        test("Deposit should fail with insufficient balance", async () => {
            // given - create token with limited balance and try to deposit more than available
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            testHelper.createTokenAccount(testTokenMint, boss, BigInt(100e9)); // Only 100 tokens
            const excessiveAmount = new BN("10000000000000000000000"); // Way more than available

            // when/then
            await expect(
                program.methods
                    .vaultDeposit(excessiveAmount)
                    .accounts({
                        tokenMint: testTokenMint,
                        state: testHelper.statePda,
                    })
                    .rpc()
            ).rejects.toThrow(); // Should fail due to insufficient balance
        });
    });

    describe("Vault Withdraw", () => {
        test("Withdraw tokens should succeed", async () => {
            // given - create unique token and set up vault with tokens
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, vaultAuthorityPda, true);
            
            // First deposit tokens to vault
            const depositAmount = new BN(100_000e9);
            await program.methods
                .vaultDeposit(depositAmount)
                .accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                })
                .rpc();
                
            const withdrawAmount = new BN(30_000e9);
            
            // Get current balances
            const initialVaultBalance = await testHelper.getTokenAccountBalance(testVaultTokenAccount);
            const initialBossBalance = await testHelper.getTokenAccountBalance(testBossTokenAccount);

            // when
            await program.methods
                .vaultWithdraw(withdrawAmount)
                .accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                })
                .rpc();

            // then
            const finalVaultBalance = await testHelper.getTokenAccountBalance(testVaultTokenAccount);
            const finalBossBalance = await testHelper.getTokenAccountBalance(testBossTokenAccount);

            expect(finalVaultBalance).toBe(initialVaultBalance - BigInt(withdrawAmount.toString()));
            expect(finalBossBalance).toBe(initialBossBalance + BigInt(withdrawAmount.toString()));
        });

        test("Withdraw should fail when not called by boss", async () => {
            // given
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const notBoss = testHelper.createUserAccount();
            const withdrawAmount = new BN(10_000e9);

            // when/then
            await expect(
                program.methods
                    .vaultWithdraw(withdrawAmount)
                    .accountsPartial({
                        tokenMint: testTokenMint,
                        state: testHelper.statePda,
                        boss: notBoss.publicKey,
                    })
                    .signers([notBoss])
                    .rpc()
            ).rejects.toThrow(); // Should fail due to boss constraint
        });

        test("Withdraw should fail with insufficient vault balance", async () => {
            // given - create unique token with small vault balance
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, vaultAuthorityPda, true);
            
            // Deposit small amount to vault
            const smallDepositAmount = new BN(1_000e9); // Only 1,000 tokens
            await program.methods
                .vaultDeposit(smallDepositAmount)
                .accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                })
                .rpc();
            
            // Try to withdraw more than vault has
            const excessiveAmount = new BN(10_000e9); // Try to withdraw 10,000 tokens

            // when/then
            await expect(
                program.methods
                    .vaultWithdraw(excessiveAmount)
                    .accounts({
                        tokenMint: testTokenMint,
                        state: testHelper.statePda,
                    })
                    .rpc()
            ).rejects.toThrow(); // Should fail due to insufficient vault balance
        });

        test("Withdraw should fail when boss token account doesn't exist", async () => {
            // given - create new mint and boss token account for deposits
            const newTokenMint = testHelper.createMint(boss, BigInt(0), 6);
            testHelper.createTokenAccount(newTokenMint, boss, BigInt(100_000e6));
            
            // Deposit some tokens first to create vault token account
            await program.methods
                .vaultDeposit(new BN(10_000e6))
                .accounts({
                    tokenMint: newTokenMint,
                    state: testHelper.statePda,
                })
                .rpc();

            // Create a different user who doesn't have a token account
            const userWithoutAccount = testHelper.createUserAccount();

            // when/then - try to withdraw to non-existent account
            await expect(
                program.methods
                    .vaultWithdraw(new BN(5_000e6))
                    .accountsPartial({
                        tokenMint: newTokenMint,
                        state: testHelper.statePda,
                        boss: userWithoutAccount.publicKey,
                    })
                    .signers([userWithoutAccount])
                    .rpc()
            ).rejects.toThrow(); // Should fail because boss token account doesn't exist
        });

        test("Withdraw should fail when vault token account doesn't exist", async () => {
            // given - create new mint and boss token account but no vault deposits
            const newTokenMint = testHelper.createMint(boss, BigInt(0), 6);
            testHelper.createTokenAccount(newTokenMint, boss, BigInt(100_000e6));

            // when/then - try to withdraw from non-existent vault account
            await expect(
                program.methods
                    .vaultWithdraw(new BN(1000e6))
                    .accounts({
                        tokenMint: newTokenMint,
                        state: testHelper.statePda,
                    })
                    .rpc()
            ).rejects.toThrow(); // Should fail because vault token account doesn't exist
        });

        test("Withdraw all remaining tokens should succeed", async () => {
            // given - create unique token and set up vault with tokens
            const testTokenMint = testHelper.createMint(boss, BigInt(0), 9);
            const testBossTokenAccount = testHelper.createTokenAccount(testTokenMint, boss, BigInt(1_000_000e9));
            const testVaultTokenAccount = getAssociatedTokenAddressSync(testTokenMint, vaultAuthorityPda, true);
            
            // First deposit tokens to vault
            const depositAmount = new BN(100_000e9);
            await program.methods
                .vaultDeposit(depositAmount)
                .accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                })
                .rpc();
                
            const vaultBalance = await testHelper.getTokenAccountBalance(testVaultTokenAccount);
            const remainingAmount = new BN(Number(vaultBalance));

            // when
            await program.methods
                .vaultWithdraw(remainingAmount)
                .accounts({
                    tokenMint: testTokenMint,
                    state: testHelper.statePda,
                })
                .rpc();

            // then
            await testHelper.expectTokenAccountAmountToBe(testVaultTokenAccount, BigInt(0));
        });
    });
});