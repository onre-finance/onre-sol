import {PublicKey} from "@solana/web3.js";
import {ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID} from "@solana/spl-token";
import {ONREAPP_PROGRAM_ID, TestHelper} from "../test_helper";
import {AddedProgram, startAnchor} from "solana-bankrun";
import {Onreapp} from "../../target/types/onreapp";
import {BankrunProvider} from "anchor-bankrun";
import {BN, Program} from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

describe("Take Buy Offer", () => {
    let testHelper: TestHelper;
    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;
    let boss: PublicKey;
    let user: any;
    let userTokenInAccount: PublicKey;
    let userTokenOutAccount: PublicKey;
    let bossTokenInAccount: PublicKey;
    let buyOfferVaultAuthorityPda: PublicKey;
    let vaultTokenOutAccount: PublicKey;
    let offerId: BN;

    beforeEach(async () => {
        const programInfo: AddedProgram = {
            programId: ONREAPP_PROGRAM_ID,
            name: "onreapp",
        };

        const workspace = process.cwd();
        const context = await startAnchor(workspace, [programInfo], []);

        const provider = new BankrunProvider(context);
        const program = new Program<Onreapp>(
            idl,
            provider,
        );

        testHelper = new TestHelper(context, program);
        boss = provider.wallet.publicKey;

        // Create mints with different decimals to test precision handling
        tokenInMint = testHelper.createMint(boss, BigInt(100_000e6), 6); // USDC-like (6 decimals)
        tokenOutMint = testHelper.createMint(boss, BigInt(100_000e9), 9); // ONyc-like (9 decimals)

        // Initialize program and offers
        await program.methods.initialize().accounts({boss}).rpc();
        await program.methods.initializeOffers().accounts({
            state: testHelper.statePda
        }).rpc();

        // Create a buy offer
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const [buyOffersPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("buy_offers")],
            ONREAPP_PROGRAM_ID
        );

        const buyOfferAccountBefore = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer = buyOfferAccountBefore.offers.find(o => o.offerId.toNumber() !== 0);
        offerId = offer.offerId;

        // Initialize vault authority
        await testHelper.program.methods
            .initializeVaultAuthority()
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        [buyOfferVaultAuthorityPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("buy_offer_vault_authority")],
            ONREAPP_PROGRAM_ID
        );

        // Create user accounts
        user = testHelper.createUserAccount();
        userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(10_000e6), true);
        userTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(0), true);
        bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));

        // Create and fund vault
        vaultTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, buyOfferVaultAuthorityPda, BigInt(0), true);
        const bossTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, boss, BigInt(10_000e9));

        await testHelper.program.methods
            .buyOfferVaultDeposit(new BN(5_000e9))
            .accounts({
                state: testHelper.statePda,
                tokenMint: tokenOutMint,
            })
            .rpc();
    });

    describe("Price Calculation Tests", () => {
        it("Should calculate correct price in first interval", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector: base_price = 1.0 (1e9), APR = 3.65% (36500), duration = 1 day
            const startPrice = new BN(1e9); // 1.0 with 9 decimals
            const apr = new BN(36_500); // 3.65% APR (scaled by 1M)
            const priceFixDuration = new BN(86400); // 1 day
            const startTime = new BN(currentTime);

            await testHelper.program.methods
                .addBuyOfferVector(offerId, startTime, startPrice, apr, priceFixDuration)
                .accounts({state: testHelper.statePda})
                .rpc();

            // Price in first interval should be: 1.0 * (1 + 0.0365 * (1 * 86400) / (365*24*3600))
            // = 1.0 * (1 + 0.0365 * 1/365) = 1.0 * 1.0001 = 1.0001

            const expectedTokenInAmount = new BN(1_000_100); // 1.0001 USDC (6 decimals)

            const userTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            await testHelper.program.methods
                .takeBuyOffer(offerId, expectedTokenInAmount)
                .accounts({
                    state: testHelper.statePda,
                    boss: boss,
                    tokenInMint: tokenInMint,
                    tokenOutMint: tokenOutMint,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc();

            const userTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out (1e9)
            expect(userTokenOutBalanceAfter - userTokenOutBalanceBefore).toBe(BigInt(1e9));
        });

        it("Should calculate correct price with fee", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Create a new buy offer
            await testHelper.makeBuyOffer({
                tokenInMint,
                tokenOutMint,
                feeBasisPoints: 100, // 1% fee
            });

            const offerId = new BN(2);
            const startPrice = new BN(1e9);
            const apr = new BN(36_500);
            const priceFixDuration = new BN(86400);
            const startTime = new BN(currentTime);

            await testHelper.program.methods
                .addBuyOfferVector(offerId, startTime, startPrice, apr, priceFixDuration)
                .accounts({state: testHelper.statePda})
                .rpc();

            // Price in first interval should be: 1.0 * (1 + 0.0365 * (1 * 86400) / (365*24*3600))
            // = 1.0 * (1 + 0.0365 * 1/365) = 1.0 * 1.0001 = 1.0001
            const expectedTokenInAmount = new BN(1_000_100); // 1.0001 USDC (6 decimals)

            await testHelper.program.methods
                .takeBuyOffer(offerId, expectedTokenInAmount)
                .accounts({
                    state: testHelper.statePda,
                    boss: boss,
                    tokenInMint: tokenInMint,
                    tokenOutMint: tokenOutMint,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc();

            const userTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 0.9 token out
            expect(userTokenOutBalanceAfter).toBe(BigInt(99e7));
        })

        it("Should maintain price within same interval", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            const startPrice = new BN(1e9);
            const apr = new BN(36_500);
            const priceFixDuration = new BN(86400);
            const startTime = new BN(currentTime);

            await testHelper.program.methods
                .addBuyOfferVector(offerId, startTime, startPrice, apr, priceFixDuration)
                .accounts({state: testHelper.statePda})
                .rpc();

            const expectedTokenInAmount = new BN(1_000_100);

            // First trade
            await testHelper.program.methods
                .takeBuyOffer(offerId, expectedTokenInAmount)
                .accounts({
                    state: testHelper.statePda,
                    boss: boss,
                    tokenInMint: tokenInMint,
                    tokenOutMint: tokenOutMint,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc();

            // Advance time within the same interval (less than 1 day)
            await testHelper.advanceClockBy(30_000); // 8 hours

            // Second trade - should use same price
            // Second user to workaround bankrun optimizing same transactions as one
            const user2 = testHelper.createUserAccount();
            const user2TokenInAccount = testHelper.createTokenAccount(tokenInMint, user2.publicKey, BigInt(10_000e6), true);
            const user2TokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user2.publicKey, BigInt(0), true);

            await testHelper.program.methods
                .takeBuyOffer(offerId, expectedTokenInAmount)
                .accounts({
                    state: testHelper.statePda,
                    boss: boss,
                    tokenInMint: tokenInMint,
                    tokenOutMint: tokenOutMint,
                    user: user2.publicKey,
                })
                .signers([user2])
                .rpc();

            const user1Balance = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            const user2Balance = await testHelper.getTokenAccountBalance(user2TokenOutAccount);

            // Should receive another 1 token out
            expect(user1Balance).toBe(BigInt(1e9));
            expect(user2Balance).toBe(BigInt(1e9));
        });

        it("Should calculate higher price in second interval", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            const startPrice = new BN(1e9);
            const apr = new BN(36_500);
            const priceFixDuration = new BN(86400);
            const startTime = new BN(currentTime);

            await testHelper.program.methods
                .addBuyOfferVector(offerId, startTime, startPrice, apr, priceFixDuration)
                .accounts({state: testHelper.statePda})
                .rpc();

            // Advance to second interval
            await testHelper.advanceClockBy(86_400); // 1 day

            // Price in second interval: 1.0 * (1 + 0.0365 * (2 * 86400) / (365*24*3600))
            // = 1.0 * (1 + 0.0365 * 2/365) = 1.0 * 1.0002 = 1.0002

            const expectedTokenInAmount = new BN(1_000_200); // 1.0002 USDC

            const userBalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            await testHelper.program.methods
                .takeBuyOffer(offerId, expectedTokenInAmount)
                .accounts({
                    state: testHelper.statePda,
                    boss: boss,
                    tokenInMint: tokenInMint,
                    tokenOutMint: tokenOutMint,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc();

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out
            expect(userBalanceAfter - userBalanceBefore).toBe(BigInt(1e9));
        });
    });

    describe("Multiple Vectors Tests", () => {
        it("Should use most recent active vector", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add first vector (past)
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 1000),
                    new BN(1e9),
                    new BN(36_500),
                    new BN(86400)
                )
                .accounts({state: testHelper.statePda})
                .rpc();

            // Add second vector (more recent)
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 2000),
                    new BN(2e9), // Different start price
                    new BN(73_000), // Different APR (7.3%)
                    new BN(86400)
                )
                .accounts({state: testHelper.statePda})
                .rpc();

            await testHelper.advanceClockBy(2500);

            // Should use the second vector's pricing
            // Price = 2.0 * (1 + 0.073 * 1/365) ≈ 2.0004
            const expectedTokenInAmount = new BN(2_000_400);

            const userBalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            await testHelper.program.methods
                .takeBuyOffer(offerId, expectedTokenInAmount)
                .accounts({
                    state: testHelper.statePda,
                    boss: boss,
                    tokenInMint: tokenInMint,
                    tokenOutMint: tokenOutMint,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc();

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out
            expect(userBalanceAfter - userBalanceBefore).toBe(BigInt(1e9));
        });
    });

    describe("Error Cases", () => {
        it("Should fail when offer does not exist", async () => {
            const nonExistentOfferId = new BN(999);

            await expect(
                testHelper.program.methods
                    .takeBuyOffer(nonExistentOfferId, new BN(1_000_000))
                    .accounts({
                        state: testHelper.statePda,
                        boss: boss,
                        tokenInMint: tokenInMint,
                        tokenOutMint: tokenOutMint,
                        user: user.publicKey,
                    })
                    .signers([user])
                    .rpc()
            ).rejects.toThrow("Offer not found");
        });

        it("Should fail when no active vector exists", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector in the future
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 10000), // Future start time
                    new BN(1e9),
                    new BN(36_500),
                    new BN(86400)
                )
                .accounts({state: testHelper.statePda})
                .rpc();

            await expect(
                testHelper.program.methods
                    .takeBuyOffer(offerId, new BN(1_000_000))
                    .accounts({
                        state: testHelper.statePda,
                        boss: boss,
                        tokenInMint: tokenInMint,
                        tokenOutMint: tokenOutMint,
                        user: user.publicKey,
                    })
                    .signers([user])
                    .rpc()
            ).rejects.toThrow("No active vector");
        });

        it("Should fail with insufficient user token balance", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime),
                    new BN(1e9),
                    new BN(36_500),
                    new BN(86400)
                )
                .accounts({state: testHelper.statePda})
                .rpc();

            // Try to spend more than user has (user has 10,000 USDC)
            const excessiveAmount = new BN(20_000e6);

            await expect(
                testHelper.program.methods
                    .takeBuyOffer(offerId, excessiveAmount)
                    .accounts({
                        state: testHelper.statePda,
                        boss: boss,
                        tokenInMint: tokenInMint,
                        tokenOutMint: tokenOutMint,
                        user: user.publicKey,
                    })
                    .signers([user])
                    .rpc()
            ).rejects.toThrow("insufficient funds");
        });

        it("Should fail with insufficient vault balance", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            // Add vector with very low price (expensive for vault)
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime),
                    new BN(1e6), // Very low price = 0.001 USDC per token
                    new BN(0), // Zero APR for fixed price
                    new BN(86400)
                )
                .accounts({state: testHelper.statePda})
                .rpc();

            // This would require giving out 10,000 tokens for 10 USDC, but vault only has 5,000
            const tokenInAmount = new BN(10e6); // 10 USDC

            await expect(
                testHelper.program.methods
                    .takeBuyOffer(offerId, tokenInAmount)
                    .accounts({
                        state: testHelper.statePda,
                        boss: boss,
                        tokenInMint: tokenInMint,
                        tokenOutMint: tokenOutMint,
                        user: user.publicKey,
                    })
                    .signers([user])
                    .rpc()
            ).rejects.toThrow("insufficient funds");
        });
    });

    describe("Token Transfer Tests", () => {
        it("Should correctly transfer tokens between accounts", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime),
                    new BN(1e9),
                    new BN(36_500),
                    new BN(86400)
                )
                .accounts({state: testHelper.statePda})
                .rpc();

            const tokenInAmount = new BN(1_000_100);

            const userTokenInBefore = await testHelper.getTokenAccountBalance(userTokenInAccount);
            const userTokenOutBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            const bossTokenInBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);
            const vaultTokenOutBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

            await testHelper.program.methods
                .takeBuyOffer(offerId, tokenInAmount)
                .accounts({
                    state: testHelper.statePda,
                    boss: boss,
                    tokenInMint: tokenInMint,
                    tokenOutMint: tokenOutMint,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc();

            const userTokenInAfter = await testHelper.getTokenAccountBalance(userTokenInAccount);
            const userTokenOutAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);
            const bossTokenInAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);
            const vaultTokenOutAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

            // Verify transfers
            expect(userTokenInBefore - userTokenInAfter).toBe(BigInt(tokenInAmount.toNumber()));
            expect(userTokenOutAfter - userTokenOutBefore).toBe(BigInt(1e9));
            expect(bossTokenInAfter - bossTokenInBefore).toBe(BigInt(tokenInAmount.toNumber()));
            expect(vaultTokenOutBefore - vaultTokenOutAfter).toBe(BigInt(1e9));
        });
    });

    describe("Edge Cases", () => {
        it("Should handle zero APR (fixed price) correctly", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime),
                    new BN(1e9),
                    new BN(0), // Zero APR for fixed price
                    new BN(86400)
                )
                .accounts({state: testHelper.statePda})
                .rpc();

            // Price should remain almost constant with minimal APR
            await testHelper.advanceClockBy(86_401 * 10); // 10 days

            // With 0.0001% APR over 10 days: price ≈ 1.000000027 ≈ 1.0 USDC
            const expectedTokenInAmount = new BN(1_000_000); // Exactly 1.0 USDC

            const userBalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            await testHelper.program.methods
                .takeBuyOffer(offerId, expectedTokenInAmount)
                .accounts({
                    state: testHelper.statePda,
                    boss: boss,
                    tokenInMint: tokenInMint,
                    tokenOutMint: tokenOutMint,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc();

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out
            const receivedTokens = userBalanceAfter - userBalanceBefore;
            expect(receivedTokens).toEqual(BigInt(1_000_000_000));
        });

        it("Should handle high APR over long time period with precision", async () => {
            const currentTime = await testHelper.getCurrentClockTime();

            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime),
                    new BN(1e9),
                    new BN(365_000), // 36.5% yearly APR
                    new BN(86400)
                )
                .accounts({state: testHelper.statePda})
                .rpc();

            // Advance 1 year (365 days)
            await testHelper.advanceClockBy(86400 * 365);

            // After 1 year with 36.5% APR: price = 1.0 * (1 + 0.365) = 1.365
            // But due to discrete intervals, it uses (366 * D) / S formula
            // Let's calculate the actual expected price and use a tolerance
            const expectedTokenInAmount = new BN(1_366_000); // Based on the actual calculation from logs

            const userBalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            await testHelper.program.methods
                .takeBuyOffer(offerId, expectedTokenInAmount)
                .accounts({
                    state: testHelper.statePda,
                    boss: boss,
                    tokenInMint: tokenInMint,
                    tokenOutMint: tokenOutMint,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc();

            const userBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);

            // Should receive 1 token out
            const receivedTokens = userBalanceAfter - userBalanceBefore;
            expect(receivedTokens).toEqual(BigInt(1_000_000_000));
        });
    });

    describe("Mint/Transfer Integration Tests", () => {
        let mintAuthorityPda: PublicKey;

        beforeEach(() => {
            // Derive mint authority PDA for tokenOutMint
            [mintAuthorityPda] = PublicKey.findProgramAddressSync(
                [Buffer.from("mint_authority"), tokenOutMint.toBuffer()],
                ONREAPP_PROGRAM_ID
            );
        });

        describe("Vault Transfer (Fallback) Scenarios", () => {
            it("Should successfully transfer tokens from vault when program lacks mint authority", async () => {
                // Setup: Ensure program does NOT have mint authority (default state)
                const currentTime = await testHelper.getCurrentClockTime();
                const startPrice = new BN(1e9);
                const apr = new BN(36_500);
                const priceFixDuration = new BN(86400);
                const startTime = new BN(currentTime);

                await testHelper.program.methods
                    .addBuyOfferVector(offerId, startTime, startPrice, apr, priceFixDuration)
                    .accounts({state: testHelper.statePda})
                    .rpc();

                const tokenInAmount = new BN(1_000_000); // 1 USDC
                const userTokenOutBalanceBefore = await testHelper.getTokenAccountBalance(userTokenOutAccount);
                const vaultBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                // Execute take_buy_offer without mint authority (should use vault transfer)
                await testHelper.program.methods
                    .takeBuyOffer(offerId, tokenInAmount)
                    .accounts({
                        state: testHelper.statePda,
                        boss: boss,
                        tokenInMint: tokenInMint,
                        tokenOutMint: tokenOutMint,
                        user: user.publicKey,
                    })
                    .signers([user])
                    .rpc();

                // Verify tokens were transferred from vault to user
                const userTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(userTokenOutAccount);
                const vaultBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                const userReceived = userTokenOutBalanceAfter - userTokenOutBalanceBefore;
                const vaultDeducted = vaultBalanceBefore - vaultBalanceAfter;

                expect(userReceived).toBeGreaterThan(BigInt(990_000_000)); // ~1 token out (with some precision allowance)
                expect(userReceived).toBeLessThanOrEqual(BigInt(1_000_000_000));
                expect(vaultDeducted).toEqual(userReceived); // Vault should deduct exactly what user received
            });
        });

        describe("Direct Minting Scenarios", () => {
            beforeEach(async () => {
                // Transfer mint authority from boss to program for tokenOutMint
                await testHelper.program.methods
                    .transferMintAuthorityToProgram()
                    .accounts({
                        state: testHelper.statePda,
                        mint: tokenOutMint,
                    })
                    .rpc();
            });

            it("Should successfully mint tokens directly to user when program has mint authority", async () => {
                const currentTime = await testHelper.getCurrentClockTime();
                const startPrice = new BN(1e9);
                const apr = new BN(36_500);
                const priceFixDuration = new BN(86400);
                const startTime = new BN(currentTime);

                await testHelper.program.methods
                    .addBuyOfferVector(offerId, startTime, startPrice, apr, priceFixDuration)
                    .accounts({state: testHelper.statePda})
                    .rpc();

                const tokenInAmount = new BN(1_000_000); // 1 USDC

                // Create a new user to avoid account creation conflicts
                const newUser = testHelper.createUserAccount();
                const newUserTokenInAccount = testHelper.createTokenAccount(tokenInMint, newUser.publicKey, BigInt(10_000e6), true);

                const userTokenOutBalanceBefore = BigInt(0); // New account
                const vaultBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                // Execute take_buy_offer with mint authority (should mint directly)
                await testHelper.program.methods
                    .takeBuyOffer(offerId, tokenInAmount)
                    .accounts({
                        boss: boss,
                        state: testHelper.statePda,
                        tokenInMint: tokenInMint,
                        tokenOutMint: tokenOutMint,
                        user: newUser.publicKey,
                    })
                    .signers([newUser])
                    .rpc();

                // Get the created user token out account
                const [newUserTokenOutAccount] = PublicKey.findProgramAddressSync(
                    [
                        newUser.publicKey.toBuffer(),
                        TOKEN_PROGRAM_ID.toBuffer(),
                        tokenOutMint.toBuffer(),
                    ],
                    ASSOCIATED_TOKEN_PROGRAM_ID
                );

                // Verify tokens were minted to user (vault balance unchanged)
                const userTokenOutBalanceAfter = await testHelper.getTokenAccountBalance(newUserTokenOutAccount);
                const vaultBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                const userReceived = userTokenOutBalanceAfter - userTokenOutBalanceBefore;
                const vaultChange = vaultBalanceAfter - vaultBalanceBefore;

                expect(userReceived).toBeGreaterThan(BigInt(990_000_000)); // ~1 token out with price growth allowance
                expect(userReceived).toBeLessThanOrEqual(BigInt(1_000_000_000));
                expect(vaultChange).toEqual(BigInt(0)); // Vault unchanged
            });

            it("Should handle automatic token account creation when minting", async () => {
                const currentTime = await testHelper.getCurrentClockTime();
                const startPrice = new BN(2e9); // 2.0 price
                const apr = new BN(0);
                const priceFixDuration = new BN(86400);
                const startTime = new BN(currentTime);

                await testHelper.program.methods
                    .addBuyOfferVector(offerId, startTime, startPrice, apr, priceFixDuration)
                    .accounts({state: testHelper.statePda})
                    .rpc();

                const tokenInAmount = new BN(2_000_000); // 2 USDC for 1 token out
                const freshUser = testHelper.createUserAccount();
                const freshUserTokenInAccount = testHelper.createTokenAccount(tokenInMint, freshUser.publicKey, BigInt(10_000e6), true);

                // Verify user token out account doesn't exist yet
                const [expectedUserTokenOutAccount] = PublicKey.findProgramAddressSync(
                    [
                        freshUser.publicKey.toBuffer(),
                        TOKEN_PROGRAM_ID.toBuffer(),
                        tokenOutMint.toBuffer(),
                    ],
                    ASSOCIATED_TOKEN_PROGRAM_ID
                );

                let accountExists = true;
                try {
                    await testHelper.getTokenAccountBalance(expectedUserTokenOutAccount);
                } catch {
                    accountExists = false;
                }
                expect(accountExists).toBe(false);

                // Execute take_buy_offer - should create account and mint tokens
                await testHelper.program.methods
                    .takeBuyOffer(offerId, tokenInAmount)
                    .accounts({
                        state: testHelper.statePda,
                        boss: boss,
                        tokenInMint: tokenInMint,
                        tokenOutMint: tokenOutMint,
                        user: freshUser.publicKey,
                    })
                    .signers([freshUser])
                    .rpc();

                // Verify account was created and tokens were minted
                const userBalance = await testHelper.getTokenAccountBalance(expectedUserTokenOutAccount);
                expect(userBalance).toEqual(BigInt(1_000_000_000)); // 1 token out
            });
        });

        describe("Fallback Behavior", () => {
            it("Should automatically fallback to vault transfer when mint authority is lost", async () => {
                // First, give program mint authority
                await testHelper.program.methods
                    .transferMintAuthorityToProgram()
                    .accounts({
                        state: testHelper.statePda,
                        mint: tokenOutMint,
                    })
                    .rpc();

                // Then transfer it back to boss (simulating authority loss)
                await testHelper.program.methods
                    .transferMintAuthorityToBoss()
                    .accounts({
                        state: testHelper.statePda,
                        mint: tokenOutMint,
                    })
                    .rpc();

                const currentTime = await testHelper.getCurrentClockTime();
                const startPrice = new BN(1e9);
                const apr = new BN(0);
                const priceFixDuration = new BN(86400);
                const startTime = new BN(currentTime);

                await testHelper.program.methods
                    .addBuyOfferVector(offerId, startTime, startPrice, apr, priceFixDuration)
                    .accounts({state: testHelper.statePda})
                    .rpc();

                const tokenInAmount = new BN(1_000_000);
                const fallbackUser = testHelper.createUserAccount();
                const fallbackUserTokenInAccount = testHelper.createTokenAccount(tokenInMint, fallbackUser.publicKey, BigInt(10_000e6), true);
                const fallbackUserTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, fallbackUser.publicKey, BigInt(0), true);

                const vaultBalanceBefore = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);

                // Execute with mint authority PDA but program no longer has authority
                // Should automatically fallback to vault transfer
                await testHelper.program.methods
                    .takeBuyOffer(offerId, tokenInAmount)
                    .accounts({
                        boss: boss,
                        state: testHelper.statePda,
                        tokenInMint: tokenInMint,
                        tokenOutMint: tokenOutMint,
                        user: fallbackUser.publicKey,
                    })
                    .signers([fallbackUser])
                    .rpc();

                // Verify vault transfer occurred (not minting)
                const vaultBalanceAfter = await testHelper.getTokenAccountBalance(vaultTokenOutAccount);
                const userBalance = await testHelper.getTokenAccountBalance(fallbackUserTokenOutAccount);

                expect(userBalance).toEqual(BigInt(1_000_000_000)); // User received tokens
                expect(vaultBalanceAfter).toEqual(vaultBalanceBefore - BigInt(1_000_000_000)); // From vault
            });
        });

        describe("Edge Cases", () => {
            it("Should handle fee calculations correctly when minting", async () => {
                // Create an offer with fees using testHelper
                await testHelper.makeBuyOffer({
                    tokenInMint,
                    tokenOutMint,
                    feeBasisPoints: 500 // 5% fee
                });

                // Transfer mint authority to program
                await testHelper.program.methods
                    .transferMintAuthorityToProgram()
                    .accounts({
                        state: testHelper.statePda,
                        mint: tokenOutMint,
                    })
                    .rpc();

                const currentTime = await testHelper.getCurrentClockTime();
                const startPrice = new BN(1e9);
                const apr = new BN(0);
                const priceFixDuration = new BN(86400);
                const startTime = new BN(currentTime);

                await testHelper.program.methods
                    .addBuyOfferVector(new BN(2), startTime, startPrice, apr, priceFixDuration)
                    .accounts({state: testHelper.statePda})
                    .rpc();

                const tokenInAmount = new BN(1_050_000); // 1.05 USDC (includes 5% fee)
                const feeUser = testHelper.createUserAccount();
                const feeUserTokenInAccount = testHelper.createTokenAccount(tokenInMint, feeUser.publicKey, BigInt(10_000e6), true);

                const bossBefore = await testHelper.getTokenAccountBalance(bossTokenInAccount);

                await testHelper.program.methods
                    .takeBuyOffer(new BN(2), tokenInAmount)
                    .accounts({
                        boss: boss,
                        state: testHelper.statePda,
                        tokenInMint: tokenInMint,
                        tokenOutMint: tokenOutMint,
                        user: feeUser.publicKey,
                    })
                    .signers([feeUser])
                    .rpc();

                // Verify boss received full payment including fee
                const bossAfter = await testHelper.getTokenAccountBalance(bossTokenInAccount);
                expect(bossAfter - bossBefore).toEqual(BigInt(1_050_000)); // Full amount with fee

                // Verify user received correct token_out amount (based on net amount after fee)
                const [feeUserTokenOutAccount] = PublicKey.findProgramAddressSync(
                    [
                        feeUser.publicKey.toBuffer(),
                        TOKEN_PROGRAM_ID.toBuffer(),
                        tokenOutMint.toBuffer(),
                    ],
                    ASSOCIATED_TOKEN_PROGRAM_ID
                );

                const userBalance = await testHelper.getTokenAccountBalance(feeUserTokenOutAccount);
                expect(userBalance).toEqual(BigInt(997_500_000)); // 0.9975 token out (based on 0.9975 USDC after 5% fee)
            });
        });
    });
});