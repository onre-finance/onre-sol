import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";
import { getMint } from "@solana/spl-token";

describe("Mint Authority Transfer", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenMint: PublicKey;
    let boss: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);
        boss = testHelper.getBoss();

        // Create a test token mint with boss as mint authority
        tokenMint = testHelper.createMint(9);

        // Initialize the program state
        await program.initialize();
    });

    describe("transfer_mint_authority_to_program", () => {
        test("Should successfully transfer mint authority from boss to program PDA", async () => {
            // when
            await program.transferMintAuthorityToProgram({ mint: tokenMint });

            // then - verify mint authority has been transferred to PDA
            const mintAccount = await getMint(program.program.provider.connection, tokenMint);
            expect(mintAccount.mintAuthority.toBase58()).toBe(program.pdas.mintAuthorityPda.toBase58());
        });

        test("Should fail if caller is not the boss", async () => {
            // given - create a different user
            const notBoss = testHelper.createUserAccount();

            // when/then
            await expect(
                program.transferMintAuthorityToProgram({ mint: tokenMint, signer: notBoss })
            ).rejects.toThrow("unknown signer");
        });

        test("Should fail if boss is not the current mint authority", async () => {
            // given - create a different mint with different authority
            const notBoss = testHelper.createUserAccount();
            const differentMint = testHelper.createMint(9, notBoss.publicKey);

            // when/then
            await expect(
                program.transferMintAuthorityToProgram({ mint: differentMint })
            ).rejects.toThrow("BossNotMintAuthority");
        });
    });

    describe("transfer_mint_authority_to_boss", () => {
        beforeEach(async () => {
            // Transfer authority to program first
            await program.transferMintAuthorityToProgram({ mint: tokenMint });
        });

        test("Should successfully transfer mint authority from program PDA back to boss", async () => {
            // when
            await program.transferMintAuthorityToBoss({ mint: tokenMint });

            // then - verify mint authority has been transferred back to boss
            const mintInfo = await testHelper.getMintInfo(tokenMint);
            expect(mintInfo.mintAuthority!.toString()).toBe(boss.toString());
        });

        test("Should fail if caller is not the boss", async () => {
            // given - create a different user
            const notBoss = testHelper.createUserAccount();

            // when/then
            await expect(
                program.transferMintAuthorityToBoss({ mint: tokenMint, signer: notBoss })
            ).rejects.toThrow("unknown signer");
        });

        test("Should fail if program PDA is not the current mint authority", async () => {
            // given - create a new mint where boss already has authority (never transferred to program)
            const newTokenMint = testHelper.createMint(9);

            // when/then - try to transfer from program PDA when boss has authority
            await expect(
                program.transferMintAuthorityToBoss({ mint: newTokenMint })
            ).rejects.toThrow("ProgramNotMintAuthority");
        });
    });

    describe("Multiple tokens support", () => {
        test("Should handle multiple different token mints independently", async () => {
            // given - create second token mint
            const token2Mint = testHelper.createMint(6);

            // when - transfer authority for both tokens to program
            await program.transferMintAuthorityToProgram({ mint: tokenMint });
            await program.transferMintAuthorityToProgram({ mint: token2Mint });

            // then - both tokens should have their respective PDAs as mint authority
            const mint1Authority = await testHelper.getMintInfo(tokenMint);
            const mint2Authority = await testHelper.getMintInfo(token2Mint);

            expect(mint1Authority.mintAuthority!.toBase58()).toBe(program.pdas.mintAuthorityPda.toBase58());
            expect(mint2Authority.mintAuthority!.toBase58()).toBe(program.pdas.mintAuthorityPda.toBase58());
        });
    });
});