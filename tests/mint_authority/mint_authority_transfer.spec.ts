import {PublicKey} from "@solana/web3.js";
import {ONREAPP_PROGRAM_ID, TestHelper} from "../test_helper";
import {AddedProgram, startAnchor} from "solana-bankrun";
import {Onreapp} from "../../target/types/onreapp";
import {BankrunProvider} from "anchor-bankrun";
import {Program} from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

describe("Mint Authority Transfer", () => {
    let testHelper: TestHelper;
    let program: Program<Onreapp>;
    let provider: BankrunProvider;

    let tokenMint: PublicKey;
    let boss: PublicKey;
    let mintAuthorityPda: PublicKey;

    beforeEach(async () => {
        const programInfo: AddedProgram = {
            programId: ONREAPP_PROGRAM_ID,
            name: "onreapp"
        };

        const workspace = process.cwd();
        const context = await startAnchor(workspace, [programInfo], []);

        provider = new BankrunProvider(context);
        program = new Program<Onreapp>(
            idl,
            provider
        );

        testHelper = new TestHelper(context, program);
        boss = provider.wallet.publicKey;

        // Create a test token mint with boss as mint authority
        tokenMint = testHelper.createMint(boss, BigInt(1_000_000e9), 9);

        // Derive mint authority PDA
        [mintAuthorityPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("mint_authority"), tokenMint.toBuffer()],
            ONREAPP_PROGRAM_ID
        );

        // Initialize the program state
        await program.methods.initialize().accounts({
            boss
        }).rpc();
    });

    describe("transfer_mint_authority_to_program", () => {
        test("Should successfully transfer mint authority from boss to program PDA", async () => {
            // when
            await program.methods
                .transferMintAuthorityToProgram()
                .accounts({
                    state: testHelper.statePda,
                    mint: tokenMint,
                })
                .rpc();

            // then - verify mint authority has been transferred to PDA
            const mintAccount = await program.provider.connection.getAccountInfo(tokenMint);
            expect(mintAccount).toBeTruthy();

            // Parse mint data to check authority
            const mintData = Buffer.from(mintAccount!.data);
            // Mint authority is at offset 4 (after option byte + supply)
            const mintAuthorityBytes = mintData.subarray(4, 36);
            const currentMintAuthority = new PublicKey(mintAuthorityBytes);

            expect(currentMintAuthority.toString()).toBe(mintAuthorityPda.toString());
        });

        test("Should fail if caller is not the boss", async () => {
            // given - create a different user
            const notBoss = testHelper.createUserAccount();

            // when/then
            await expect(program.methods
                .transferMintAuthorityToProgram()
                .accounts({
                    state: testHelper.statePda,
                    mint: tokenMint,
                })
                .signers([notBoss])
                .rpc()).rejects.toThrow("unknown signer");
        });

        test("Should fail if boss is not the current mint authority", async () => {
            // given - create a different mint with different authority
            const notBoss = testHelper.createUserAccount();
            const differentMint = testHelper.createMint(notBoss.publicKey, BigInt(1_000_000e9), 9);

            // when/then
            await expect(program.methods
                .transferMintAuthorityToProgram()
                .accounts({
                    state: testHelper.statePda,
                    mint: differentMint,
                })
                .rpc()).rejects.toThrow("BossNotMintAuthority");
        });
    });

    describe("transfer_mint_authority_to_boss", () => {
        beforeEach(async () => {
            // Transfer authority to program first
            await program.methods
                .transferMintAuthorityToProgram()
                .accounts({
                    state: testHelper.statePda,
                    mint: tokenMint,
                })
                .rpc();
        });

        test("Should successfully transfer mint authority from program PDA back to boss", async () => {
            // when
            await program.methods
                .transferMintAuthorityToBoss()
                .accounts({
                    state: testHelper.statePda,
                    mint: tokenMint,
                })
                .rpc();

            // then - verify mint authority has been transferred back to boss
            const mintAccount = await program.provider.connection.getAccountInfo(tokenMint);
            expect(mintAccount).toBeTruthy();

            // Parse mint data to check authority
            const mintData = Buffer.from(mintAccount!.data);
            const mintAuthorityBytes = mintData.subarray(4, 36);
            const currentMintAuthority = new PublicKey(mintAuthorityBytes);

            expect(currentMintAuthority.toString()).toBe(boss.toString());
        });

        test("Should fail if caller is not the boss", async () => {
            // given - create a different user
            const notBoss = testHelper.createUserAccount();

            // when/then
            await expect(program.methods
                .transferMintAuthorityToBoss()
                .accounts({
                    state: testHelper.statePda,
                    mint: tokenMint
                })
                .signers([notBoss])
                .rpc()).rejects.toThrow("unknown signer");
        });

        test("Should fail if program PDA is not the current mint authority", async () => {
            // given - create a new mint where boss already has authority (never transferred to program)
            const newTokenMint = testHelper.createMint(boss, BigInt(1_000_000e9), 9);

            // when/then - try to transfer from program PDA when boss has authority
            await expect(program.methods
                .transferMintAuthorityToBoss()
                .accounts({
                    state: testHelper.statePda,
                    mint: newTokenMint,
                })
                .rpc()).rejects.toThrow("ProgramNotMintAuthority");
        });
    });

    describe("Multiple tokens support", () => {
        test("Should handle multiple different token mints independently", async () => {
            // given - create second token mint
            const token2Mint = testHelper.createMint(boss, BigInt(2_000_000e9), 6);
            const [token2MintAuthorityPda] = PublicKey.findProgramAddressSync(
                [Buffer.from("mint_authority"), token2Mint.toBuffer()],
                ONREAPP_PROGRAM_ID
            );

            // when - transfer authority for both tokens to program
            await program.methods
                .transferMintAuthorityToProgram()
                .accounts({
                    state: testHelper.statePda,
                    mint: tokenMint
                })
                .rpc();

            await program.methods
                .transferMintAuthorityToProgram()
                .accounts({
                    state: testHelper.statePda,
                    mint: token2Mint
                })
                .rpc();

            // then - both tokens should have their respective PDAs as mint authority
            const mint1Account = await program.provider.connection.getAccountInfo(tokenMint);
            const mint2Account = await program.provider.connection.getAccountInfo(token2Mint);

            const mint1Data = Buffer.from(mint1Account!.data);
            const mint2Data = Buffer.from(mint2Account!.data);

            const mint1Authority = new PublicKey(mint1Data.subarray(4, 36));
            const mint2Authority = new PublicKey(mint2Data.subarray(4, 36));

            expect(mint1Authority.toString()).toBe(mintAuthorityPda.toString());
            expect(mint2Authority.toString()).toBe(token2MintAuthorityPda.toString());

            // Verify PDAs are different
            expect(mintAuthorityPda.toString()).not.toBe(token2MintAuthorityPda.toString());
        });
    });
});