import { Keypair, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { TestHelper } from "../test_helper.ts";
import { OnreProgram } from "../onre_program.ts";

describe("Configure Max Supply and Enforcement", () => {
    describe("Configure Max Supply Instruction", () => {
        let testHelper: TestHelper;
        let program: OnreProgram;
        let nonBoss: Keypair;
        let onycMint: PublicKey;
        let usdcMint: PublicKey;

        beforeEach(async () => {
            testHelper = await TestHelper.create();
            program = new OnreProgram(testHelper.context);

            nonBoss = testHelper.createUserAccount();
            onycMint = testHelper.createMint(9);
            usdcMint = testHelper.createMint(6);

            // Initialize state with ONyc mint
            await program.initialize({ onycMint: onycMint });
            await program.initializeMintAuthority();
            await program.initializeVaultAuthority();
        });

        test("Boss can configure max supply", async () => {
            const maxSupply = 1_000_000_000_000_000; // 1 million ONyc (9 decimals)

            await program.configureMaxSupply({ maxSupply });

            // Verify state was updated
            const state = await program.program.account.state.fetch(program.pdas.statePda);
            expect(state.maxSupply.toNumber()).toBe(maxSupply);
        });

        test("Boss can set max supply to zero (no cap)", async () => {
            // First set a cap
            await program.configureMaxSupply({ maxSupply: 1_000_000 });

            // Then remove it
            await program.configureMaxSupply({ maxSupply: 0 });

            const state = await program.program.account.state.fetch(program.pdas.statePda);
            expect(state.maxSupply.toNumber()).toBe(0);
        });

        test("Non-boss cannot configure max supply", async () => {
            await expect(
                program.configureMaxSupply({ maxSupply: 1_000_000, signer: nonBoss })
            ).rejects.toThrow();
        });

        test("Boss can update max supply multiple times", async () => {
            await program.configureMaxSupply({ maxSupply: 1_000_000 });
            let state = await program.program.account.state.fetch(program.pdas.statePda);
            expect(state.maxSupply.toNumber()).toBe(1_000_000);

            await program.configureMaxSupply({ maxSupply: 2_000_000 });
            state = await program.program.account.state.fetch(program.pdas.statePda);
            expect(state.maxSupply.toNumber()).toBe(2_000_000);
        });
    });

    describe("Mint To Enforcement", () => {
        let testHelper: TestHelper;
        let program: OnreProgram;
        let nonBoss: Keypair;
        let onycMint: PublicKey;
        let usdcMint: PublicKey;

        beforeEach(async () => {
            testHelper = await TestHelper.create();
            program = new OnreProgram(testHelper.context);

            nonBoss = testHelper.createUserAccount();
            onycMint = testHelper.createMint(9, testHelper.getBoss(), BigInt(0));
            usdcMint = testHelper.createMint(6, testHelper.getBoss(), BigInt(0));

            // Initialize state with ONyc mint
            await program.initialize({ onycMint: onycMint });
            await program.initializeMintAuthority();
            await program.initializeVaultAuthority();
            await program.transferMintAuthorityToProgram({ mint: onycMint });
        });

        test("Cannot mint ONyc beyond max supply cap", async () => {
            const maxSupply = 1_000_000_000; // 1 ONyc
            await program.configureMaxSupply({ maxSupply });

            // Try to mint more than the cap
            await expect(
                program.mintTo({ amount: maxSupply + 1 })
            ).rejects.toThrow("Minting would exceed maximum supply cap");
        });

        test("Can mint exactly up to max supply cap", async () => {
            const maxSupply = 1_000_000_000; // 1 ONyc
            await program.configureMaxSupply({ maxSupply });

            await program.mintTo({ amount: maxSupply });

            const bossOnycAccount = getAssociatedTokenAddressSync(
                onycMint,
                testHelper.getBoss(),
                false,
                TOKEN_PROGRAM_ID
            );
            const balance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(balance).toBe(BigInt(maxSupply));
        });

        test("Can mint multiple times until reaching cap", async () => {
            const maxSupply = 1_000_000_000; // 1 ONyc
            await program.configureMaxSupply({ maxSupply });

            // Mint 0.3 ONyc three times
            const mintAmount = 300_000_000;
            await program.mintTo({ amount: mintAmount });
            await testHelper.advanceSlot();
            await program.mintTo({ amount: mintAmount });
            await testHelper.advanceSlot();
            await program.mintTo({ amount: mintAmount });
            await testHelper.advanceSlot();

            // Fourth mint should fail
            await expect(
                program.mintTo({ amount: mintAmount })
            ).rejects.toThrow("Minting would exceed maximum supply cap");
        });

        test("Can mint without limits when max supply is zero", async () => {
            await program.configureMaxSupply({ maxSupply: 0 });

            const largeAmount = 1_000_000_000_000_000; // 1 million ONyc
            await program.mintTo({ amount: largeAmount });

            const bossOnycAccount = getAssociatedTokenAddressSync(
                onycMint,
                testHelper.getBoss(),
                false,
                TOKEN_PROGRAM_ID
            );
            const balance = await testHelper.getTokenAccountBalance(bossOnycAccount);
            expect(balance).toBe(BigInt(largeAmount));
        });
    });

    describe("Take Offer Enforcement", () => {
        let user: Keypair;
        let testHelper: TestHelper;
        let program: OnreProgram;
        let onycMint: PublicKey;
        let usdcMint: PublicKey;

        beforeEach(async () => {
            testHelper = await TestHelper.create();
            program = new OnreProgram(testHelper.context);

            onycMint = testHelper.createMint(9, testHelper.getBoss(), BigInt(0));
            usdcMint = testHelper.createMint(6, testHelper.getBoss(), BigInt(0));

            user = testHelper.createUserAccount();

            // Set up token accounts and balances
            testHelper.createTokenAccount(usdcMint, testHelper.getBoss(), BigInt(0));
            testHelper.createTokenAccount(usdcMint, user.publicKey, BigInt(1_000_000_000)); // 1000 USDC

            // Create vault accounts
            testHelper.createTokenAccount(usdcMint, program.pdas.offerVaultAuthorityPda, BigInt(0), true);
            testHelper.createTokenAccount(onycMint, program.pdas.offerVaultAuthorityPda, BigInt(0), true);

            // Initialize state with ONyc mint
            await program.initialize({ onycMint: onycMint });
            await program.initializeMintAuthority();
            await program.initializeVaultAuthority();

            // Transfer mint authority to program
            await program.transferMintAuthorityToProgram({ mint: onycMint });

            // Create and configure offer
            await program.makeOffer({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                feeBasisPoints: 0
            });

            await program.addOfferVector({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                startTime: Math.floor(Date.now() / 1000) - 1000,
                startPrice: 1_000_000_000, // 1:1 price (9 decimals)
                apr: 0,
                priceFixDuration: 3600
            });
        });

        test("Cannot take offer if minting would exceed max supply", async () => {
            const maxSupply = 500_000_000; // 0.5 ONyc
            await program.configureMaxSupply({ maxSupply });

            // Try to take offer for 1 ONyc (would exceed 0.5 cap)
            await expect(
                program.takeOffer({
                    tokenInMint: usdcMint,
                    tokenOutMint: onycMint,
                    tokenInAmount: 1_000_000, // 1 USDC = 1 ONyc
                    user: user.publicKey,
                    signer: user
                })
            ).rejects.toThrow("Minting would exceed maximum supply cap");
        });

        test("Can take offer within max supply cap", async () => {
            const maxSupply = 2_000_000_000; // 2 ONyc
            await program.configureMaxSupply({ maxSupply });

            // Take offer for 1 ONyc
            await program.takeOffer({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                tokenInAmount: 1_000_000, // 1 USDC
                user: user.publicKey,
                signer: user
            });

            const userOnycAccount = getAssociatedTokenAddressSync(
                onycMint,
                user.publicKey,
                false,
                TOKEN_PROGRAM_ID
            );
            const balance = await testHelper.getTokenAccountBalance(userOnycAccount);
            expect(balance).toBeGreaterThan(BigInt(0));
        });

        test("Multiple users can take offers until cap is reached", async () => {
            const maxSupply = 1_000_000_000; // 1 ONyc
            await program.configureMaxSupply({ maxSupply });

            const user1 = testHelper.createUserAccount();
            const user2 = testHelper.createUserAccount();

            testHelper.createTokenAccount(usdcMint, user1.publicKey, BigInt(1_000_000)); // 1 USDC
            testHelper.createTokenAccount(usdcMint, user2.publicKey, BigInt(1_000_000)); // 1 USDC

            // User1 takes 0.6 ONyc
            await program.takeOffer({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                tokenInAmount: 600_000, // 0.6 USDC
                user: user1.publicKey,
                signer: user1
            });

            // User2 can take 0.4 ONyc
            await program.takeOffer({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                tokenInAmount: 400_000, // 0.4 USDC
                user: user2.publicKey,
                signer: user2
            });

            // User2 cannot take more
            await expect(
                program.takeOffer({
                    tokenInMint: usdcMint,
                    tokenOutMint: onycMint,
                    tokenInAmount: 1, // Even 1 more
                    user: user2.publicKey,
                    signer: user2
                })
            ).rejects.toThrow("Minting would exceed maximum supply cap");
        });
    });

    describe("Take Offer Permissionless Enforcement", () => {
        let user: Keypair;
        let testHelper: TestHelper;
        let program: OnreProgram;
        let onycMint: PublicKey;
        let usdcMint: PublicKey;

        beforeEach(async () => {
            testHelper = await TestHelper.create();
            program = new OnreProgram(testHelper.context);

            onycMint = testHelper.createMint(9, testHelper.getBoss(), BigInt(0));
            usdcMint = testHelper.createMint(6, testHelper.getBoss(), BigInt(0));

            user = testHelper.createUserAccount();


            // Set up token accounts
            testHelper.createTokenAccount(usdcMint, testHelper.getBoss(), BigInt(0));
            testHelper.createTokenAccount(usdcMint, user.publicKey, BigInt(1_000_000_000));

            // Create intermediary accounts (PDA as owner requires allowOwnerOffCurve)
            testHelper.createTokenAccount(usdcMint, program.pdas.permissionlessAuthorityPda, BigInt(0), true);
            testHelper.createTokenAccount(onycMint, program.pdas.permissionlessAuthorityPda, BigInt(0), true);

            // Create vault accounts
            testHelper.createTokenAccount(usdcMint, program.pdas.offerVaultAuthorityPda, BigInt(0), true);
            testHelper.createTokenAccount(onycMint, program.pdas.offerVaultAuthorityPda, BigInt(0), true);

            // Initialize state with ONyc mint
            await program.initialize({ onycMint: onycMint });
            await program.initializeMintAuthority();
            await program.initializeVaultAuthority();
            // Initialize permissionless authority
            await program.initializePermissionlessAuthority({ accountName: "test" });

            // Transfer mint authority
            await program.transferMintAuthorityToProgram({ mint: onycMint });

            // Create offer with permissionless enabled
            await program.makeOffer({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                feeBasisPoints: 0,
                allowPermissionless: true
            });

            await program.addOfferVector({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                startTime: Math.floor(Date.now() / 1000) - 1000,
                startPrice: 1_000_000_000,
                apr: 0,
                priceFixDuration: 3600
            });
        });

        test("Cannot take permissionless offer if minting would exceed max supply", async () => {
            const maxSupply = 500_000_000; // 0.5 ONyc
            await program.configureMaxSupply({ maxSupply });

            await expect(
                program.takeOfferPermissionless({
                    tokenInMint: usdcMint,
                    tokenOutMint: onycMint,
                    tokenInAmount: 1_000_000, // 1 USDC
                    user: user.publicKey,
                    signer: user
                })
            ).rejects.toThrow("Minting would exceed maximum supply cap");
        });

        test("Can take permissionless offer within max supply cap", async () => {
            const maxSupply = 2_000_000_000; // 2 ONyc
            await program.configureMaxSupply({ maxSupply });

            await program.takeOfferPermissionless({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                tokenInAmount: 1_000_000,
                user: user.publicKey,
                signer: user
            });

            const userOnycAccount = getAssociatedTokenAddressSync(
                onycMint,
                user.publicKey,
                false,
                TOKEN_PROGRAM_ID
            );
            const balance = await testHelper.getTokenAccountBalance(userOnycAccount);
            expect(balance).toBeGreaterThan(BigInt(0));
        });

        test("Permissionless offer respects cumulative supply from all minting operations", async () => {
            const maxSupply = 1_500_000_000; // 1.5 ONyc
            await program.configureMaxSupply({ maxSupply });

            // First mint 1 ONyc via mint_to
            await program.mintTo({ amount: 1_000_000_000 });

            // Then try to mint 0.6 ONyc via permissionless offer (should fail)
            await expect(
                program.takeOfferPermissionless({
                    tokenInMint: usdcMint,
                    tokenOutMint: onycMint,
                    tokenInAmount: 600_000,
                    user: user.publicKey,
                    signer: user
                })
            ).rejects.toThrow("Minting would exceed maximum supply cap");

            // But 0.5 ONyc should work
            await program.takeOfferPermissionless({
                tokenInMint: usdcMint,
                tokenOutMint: onycMint,
                tokenInAmount: 500_000,
                user: user.publicKey,
                signer: user
            });
        });
    });
});
