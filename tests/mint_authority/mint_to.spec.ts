import { Keypair, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { TestHelper } from "../test_helper.ts";
import { OnreProgram } from "../onre_program.ts";

describe("Mint To", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let nonBoss: Keypair;
    let onycMint: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        nonBoss = testHelper.createUserAccount();
        onycMint = testHelper.createMint(9);

        // Initialize state with ONyc mint
        await program.initialize({ onycMint: onycMint });
    });

    test("Boss can mint ONyc tokens to their account after transferring mint authority", async () => {
        // given - transfer mint authority to program
        await program.transferMintAuthorityToProgram({ mint: onycMint });

        // Create the boss ONyc token account
        const bossOnycAccount = getAssociatedTokenAddressSync(
            onycMint,
            testHelper.getBoss(),
            false,
            TOKEN_PROGRAM_ID
        );
        testHelper.createTokenAccount(onycMint, testHelper.getBoss(), BigInt(0));

        const amountToMint = 1000000000; // 1 ONyc token (9 decimals)

        // when - boss mints tokens
        await program.mintTo({ amount: amountToMint });

        // then - boss should have the minted tokens
        const accountBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
        expect(accountBalance).toBe(BigInt(amountToMint));
    });

    test("Non-boss cannot mint ONyc tokens - should fail", async () => {
        // given - transfer mint authority to program
        await program.transferMintAuthorityToProgram({ mint: onycMint });

        // when & then
        await expect(
            program.mintTo({ amount: 1000000000, signer: nonBoss })
        ).rejects.toThrow();
    });

    test("Cannot mint without program having mint authority - should fail", async () => {
        // when & then - try to mint without transferring mint authority first
        await expect(
            program.mintTo({ amount: 1000000000 })
        ).rejects.toThrow("Program does not have mint authority for this token");
    });

    test("Cannot mint with different mint than stored in state - should fail", async () => {
        // given - transfer mint authority for original mint to program
        await program.transferMintAuthorityToProgram({ mint: onycMint });

        // Create a different mint and set it as the onyc_mint in state
        const differentMint = testHelper.createMint(9);
        await program.setOnycMint({ onycMint: differentMint });

        // when & then - try to mint (program has authority for original mint but state expects different mint)
        await expect(
            program.mintTo({ amount: 1000000000 })
        ).rejects.toThrow("Program does not have mint authority for this token");
    });

    test("Can mint multiple times and amounts accumulate", async () => {
        // given - transfer mint authority to program
        await program.transferMintAuthorityToProgram({ mint: onycMint });

        const firstAmount = 500000000; // 0.5 ONyc
        const secondAmount = 300000000; // 0.3 ONyc
        const expectedTotal = firstAmount + secondAmount; // 0.8 ONyc

        // when - mint twice
        await program.mintTo({ amount: firstAmount });
        await program.mintTo({ amount: secondAmount });

        // then - amounts should accumulate
        const bossOnycAccount = getAssociatedTokenAddressSync(
            onycMint,
            testHelper.getBoss(),
            false,
            TOKEN_PROGRAM_ID
        );
        const accountBalance = await testHelper.getTokenAccountBalance(bossOnycAccount);
        expect(accountBalance).toBe(BigInt(expectedTotal));
    });

    test("Mint creates boss token account if it doesn't exist", async () => {
        // given - transfer mint authority to program
        await program.transferMintAuthorityToProgram({ mint: onycMint });

        const bossOnycAccount = getAssociatedTokenAddressSync(
            onycMint,
            testHelper.getBoss(),
            false,
            TOKEN_PROGRAM_ID
        );

        // Verify account doesn't exist initially
        let accountExists = true;
        try {
            await testHelper.getTokenAccount(bossOnycAccount);
        } catch {
            accountExists = false;
        }
        expect(accountExists).toBe(false);

        // when - mint tokens (should create account)
        await program.mintTo({ amount: 1000000000 });

        // then - account should be created and have the minted tokens
        const accountInfo = await testHelper.getTokenAccount(bossOnycAccount);
        expect(accountInfo.amount).toBe(BigInt(1000000000));
        expect(accountInfo.mint).toEqual(onycMint);
        expect(accountInfo.owner).toEqual(testHelper.getBoss());
    });

    test("Works with different ONyc mint after updating state", async () => {
        // given - transfer authority for original mint
        await program.transferMintAuthorityToProgram({ mint: onycMint });

        // Create new mint and update state
        const newOnycMint = testHelper.createMint(9);
        await program.setOnycMint({ onycMint: newOnycMint });
        await program.transferMintAuthorityToProgram({ mint: newOnycMint });

        // when - mint from new ONyc mint
        await program.mintTo({ amount: 500000000 });

        // then - should mint from new mint to boss account
        const bossNewOnycAccount = getAssociatedTokenAddressSync(
            newOnycMint,
            testHelper.getBoss(),
            false,
            TOKEN_PROGRAM_ID
        );
        const accountInfo = await testHelper.getTokenAccount(bossNewOnycAccount);
        expect(accountInfo.amount).toBe(BigInt(500000000));
        expect(accountInfo.mint).toEqual(newOnycMint);
    });
});