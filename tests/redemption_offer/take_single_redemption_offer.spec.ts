import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

describe("Take single redemption offer", () => {
    let testHelper: TestHelper;
    let program: Program<Onreapp>;
    
    let boss: PublicKey;
    let user: any;
    let singleRedemptionOfferAccountPda: PublicKey;
    let vaultAuthorityPda: PublicKey;

    beforeAll(async () => {
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
        user = testHelper.createUserAccount();
        
        // Initialize program, offers, and vault authority
        await program.methods.initialize().accounts({ boss }).rpc();
        await program.methods.initializeOffers().accounts({ 
            state: testHelper.statePda 
        }).rpc();
        await program.methods.initializeVaultAuthority().accounts({ 
            state: testHelper.statePda 
        }).rpc();

        // Get PDAs
        [singleRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from('single_redemption_offers')], ONREAPP_PROGRAM_ID);
        [vaultAuthorityPda] = PublicKey.findProgramAddressSync([Buffer.from('vault_authority')], ONREAPP_PROGRAM_ID);

        // Create some test tokens and deposit to vault for all tests to use
        // Token with 9 decimals - deposit 10,000 tokens
        const token9Mint = testHelper.createMint(boss, BigInt(0), 9);
        testHelper.createTokenAccount(token9Mint, boss, BigInt(10000e9));
        await program.methods
            .vaultDeposit(new BN("10000000000000")) // 10,000 tokens
            .accounts({
                tokenMint: token9Mint,
                state: testHelper.statePda,
            })
            .rpc();

        // Token with 6 decimals - deposit 10,000 tokens  
        const token6Mint = testHelper.createMint(boss, BigInt(0), 6);
        testHelper.createTokenAccount(token6Mint, boss, BigInt(10000e6));
        await program.methods
            .vaultDeposit(new BN("10000000000")) // 10,000 tokens
            .accounts({
                tokenMint: token6Mint,
                state: testHelper.statePda,
            })
            .rpc();

        // Store mints for tests to use
        (global as any).testToken9Mint = token9Mint;
        (global as any).testToken6Mint = token6Mint;
    });

    test("Take redemption offer with same decimals (9,9) should succeed", async () => {
        // given - use pre-deposited tokens
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint = (global as any).testToken9Mint; // Use pre-deposited token
        
        // Create user accounts and fund them
        const userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(1000e9)); // 1000 tokens
        const userTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(0));
        const bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        const vaultTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, vaultAuthorityPda, true);

        // Create redemption offer: price = 2.0 (2000000000 with 9 decimals)
        const price = new BN("2000000000"); // 2.0 * 10^9
        const startTime = new BN(Math.floor(Date.now() / 1000) - 60); // Start 1 minute ago
        const endTime = new BN(startTime.toNumber() + 3600);

        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        // Get offer ID
        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const offerId = redemptionOfferAccountData.counter.toNumber();

        // when - user takes offer: pays 10 token_in, should get 5 token_out (10 / 2 = 5)
        const tokenInAmount = new BN("10000000000"); // 10 tokens with 9 decimals

        await program.methods
            .takeSingleRedemptionOffer(new BN(offerId), tokenInAmount)
            .accountsPartial({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
                boss,
                user: user.publicKey,
            })
            .signers([user])
            .rpc();

        // then - verify balances
        // User should have: 1000 - 10 = 990 token_in, 0 + 5 = 5 token_out
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt("990000000000"));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount, BigInt("5000000000"));
        
        // Boss should have: 0 + 10 = 10 token_in
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt("10000000000"));
        
        // Vault should have: 10,000 - 5 = 9,995 token_out
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount, BigInt("9995000000000"));
    });

    test("Take large redemption offer (20 million tokens) should succeed", async () => {
        // given - create tokens for large redemption
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint = testHelper.createMint(boss, BigInt(0), 9);
        
        // Create user accounts and fund them with enough tokens
        const userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(40_000_000e9)); // 40 million tokens
        const userTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(0));
        const bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        
        // Deposit 20 million tokens to vault for this test
        testHelper.createTokenAccount(tokenOutMint, boss, BigInt(20_000_000e9)); // 20 million tokens
        await program.methods
            .vaultDeposit(new BN("20000000000000000")) // 20 million tokens
            .accounts({
                tokenMint: tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        const vaultTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, vaultAuthorityPda, true);

        // Create redemption offer: price = 2.0 (user pays 2 tokens to get 1 token out)
        const price = new BN("2000000000"); // 2.0 * 10^9
        const startTime = new BN(Math.floor(Date.now() / 1000) - 60); // Start 1 minute ago
        const endTime = new BN(startTime.toNumber() + 3600);

        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        // Get offer ID
        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const offerId = redemptionOfferAccountData.counter.toNumber();

        // when - user redeems 20 million tokens: pays 40 million token_in, gets 20 million token_out
        const tokenInAmount = new BN("40000000000000000"); // 40 million tokens with 9 decimals

        await program.methods
            .takeSingleRedemptionOffer(new BN(offerId), tokenInAmount)
            .accountsPartial({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
                boss,
                user: user.publicKey,
            })
            .signers([user])
            .rpc();

        // then - verify balances
        // User should have: 40M - 40M = 0 token_in, 0 + 20M = 20M token_out
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt("0"));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount, BigInt("20000000000000000"));
        
        // Boss should have: 0 + 40M = 40M token_in
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt("40000000000000000"));
        
        // Vault should have: 20M - 20M = 0 token_out
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount, BigInt("0"));
    });

    test("Take redemption offer with different decimals (6,9) should succeed", async () => {
        // given - token_in has 6 decimals, token_out has 9 decimals
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 6);
        const tokenOutMint = testHelper.createMint(boss, BigInt(0), 9);
        
        // Create user accounts and fund them
        const userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(1000e6)); // 1000 tokens
        const userTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(0));
        const bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        
        // Create boss token account for token_out and deposit to vault
        testHelper.createTokenAccount(tokenOutMint, boss, BigInt(500e9)); // 500 tokens
        const vaultTokenOutAccount = getAssociatedTokenAddressSync(tokenOutMint, vaultAuthorityPda, true);
        
        // Deposit tokens to vault
        await program.methods
            .vaultDeposit(new BN("500000000000")) // 500 tokens
            .accounts({
                tokenMint: tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        // Create redemption offer: price = 3.0 (3000000000 with 9 decimals)
        const price = new BN("3000000000"); // 3.0 * 10^9
        const startTime = new BN(Math.floor(Date.now() / 1000) - 60); // Start 1 minute ago
        const endTime = new BN(startTime.toNumber() + 3600);

        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        // Get offer ID
        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const offerId = redemptionOfferAccountData.counter.toNumber();

        // when - user takes offer: pays 15 token_in (6 decimals), should get 5 token_out (9 decimals)
        // Calculation: (15 * 10^6) * 10^(9+9) / (3 * 10^9 * 10^6) = 15 * 10^24 / (3 * 10^15) = 5 * 10^9
        const tokenInAmount = new BN("15000000"); // 15 tokens with 6 decimals

        await program.methods
            .takeSingleRedemptionOffer(new BN(offerId), tokenInAmount)
            .accountsPartial({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
                boss,
                user: user.publicKey,
            })
            .signers([user])
            .rpc();

        // then - verify balances
        // User should have: 1000 - 15 = 985 token_in, 0 + 5 = 5 token_out
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt("985000000"));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount, BigInt("5000000000"));
        
        // Boss should have: 0 + 15 = 15 token_in
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt("15000000"));
        
        // Vault should have: 500 - 5 = 495 token_out
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount, BigInt("495000000000"));
    });

    test("Take redemption offer with different decimals (9,6) should succeed", async () => {
        // given - token_in has 9 decimals, token_out has 6 decimals
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint = testHelper.createMint(boss, BigInt(0), 6);
        
        // Create accounts and fund them
        const userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(1000e9)); // 1000 tokens
        const userTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(0));
        const bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        const vaultTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, vaultAuthorityPda, BigInt(500e6), true); // 500 tokens

        // Create redemption offer: price = 0.5 (500000000 with 9 decimals)
        const price = new BN("500000000"); // 0.5 * 10^9
        const startTime = new BN(Math.floor(Date.now() / 1000) - 60); // Start 1 minute ago
        const endTime = new BN(startTime.toNumber() + 3600);

        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        // Get offer ID
        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const offerId = redemptionOfferAccountData.counter.toNumber();

        // when - user takes offer: pays 1 token_in (9 decimals), should get 2 token_out (6 decimals)
        // Calculation: (1 * 10^9) * 10^(6+9) / (0.5 * 10^9 * 10^9) = 1 * 10^24 / (0.5 * 10^18) = 2 * 10^6
        const tokenInAmount = new BN("1000000000"); // 1 token with 9 decimals

        await program.methods
            .takeSingleRedemptionOffer(new BN(offerId), tokenInAmount)
            .accountsPartial({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
                boss,
                user: user.publicKey,
            })
            .signers([user])
            .rpc();

        // then - verify balances
        // User should have: 1000 - 1 = 999 token_in, 0 + 2 = 2 token_out
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt("999000000000"));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount, BigInt("2000000"));
        
        // Boss should have: 0 + 1 = 1 token_in
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt("1000000000"));
        
        // Vault should have: 500 - 2 = 498 token_out
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount, BigInt("498000000"));
    });

    test("Take redemption offer with fractional price should succeed", async () => {
        // given - both tokens have 9 decimals, price = 1.5
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint = testHelper.createMint(boss, BigInt(0), 9);
        
        // Create accounts and fund them
        const userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(1000e9)); // 1000 tokens
        const userTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(0));
        const bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        const vaultTokenOutAccount = testHelper.createTokenAccount(tokenOutMint, vaultAuthorityPda, BigInt(500e9), true); // 500 tokens

        // Create redemption offer: price = 1.5 (1500000000 with 9 decimals)
        const price = new BN("1500000000"); // 1.5 * 10^9
        const startTime = new BN(Math.floor(Date.now() / 1000) - 60); // Start 1 minute ago
        const endTime = new BN(startTime.toNumber() + 3600);

        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        // Get offer ID
        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const offerId = redemptionOfferAccountData.counter.toNumber();

        // when - user takes offer: pays 3 token_in, should get 2 token_out (3 / 1.5 = 2)
        const tokenInAmount = new BN("3000000000"); // 3 tokens with 9 decimals

        await program.methods
            .takeSingleRedemptionOffer(new BN(offerId), tokenInAmount)
            .accountsPartial({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
                boss,
                user: user.publicKey,
            })
            .signers([user])
            .rpc();

        // then - verify balances
        // User should have: 1000 - 3 = 997 token_in, 0 + 2 = 2 token_out
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt("997000000000"));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount, BigInt("2000000000"));
        
        // Boss should have: 0 + 3 = 3 token_in
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt("3000000000"));
        
        // Vault should have: 500 - 2 = 498 token_out
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount, BigInt("498000000000"));
    });

    test("Take redemption offer should fail when offer doesn't exist", async () => {
        // given
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint = testHelper.createMint(boss, BigInt(0), 9);
        
        testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(100e9));
        testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(0));
        testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        testHelper.createTokenAccount(tokenOutMint, vaultAuthorityPda, BigInt(100e9), true);

        // when/then - try to take non-existent offer
        await expect(
            program.methods
                .takeSingleRedemptionOffer(new BN(9999), new BN("1000000000"))
                .accounts({
                    tokenInMint,
                    tokenOutMint,
                    state: testHelper.statePda,
                    boss,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc()
        ).rejects.toThrow("Offer not found");
    });

    test("Take redemption offer should fail with wrong token mints", async () => {
        // given - create offer with specific mints
        const correctTokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const correctTokenOutMint = testHelper.createMint(boss, BigInt(0), 9);
        const wrongTokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        
        testHelper.createTokenAccount(correctTokenInMint, boss, BigInt(0));
        testHelper.createTokenAccount(correctTokenOutMint, vaultAuthorityPda, BigInt(100e9), true);
        testHelper.createTokenAccount(wrongTokenInMint, user.publicKey, BigInt(100e9));
        testHelper.createTokenAccount(correctTokenOutMint, user.publicKey, BigInt(0));
        testHelper.createTokenAccount(wrongTokenInMint, boss, BigInt(0));

        const price = new BN("1000000000");
        const startTime = new BN(Math.floor(Date.now() / 1000) - 60); // Start 1 minute ago
        const endTime = new BN(startTime.toNumber() + 3600);

        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint: correctTokenInMint,
                tokenOutMint: correctTokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const offerId = redemptionOfferAccountData.counter.toNumber();

        // when/then - try to take offer with wrong token_in mint
        await expect(
            program.methods
                .takeSingleRedemptionOffer(new BN(offerId), new BN("1000000000"))
                .accounts({
                    tokenInMint: wrongTokenInMint,
                    tokenOutMint: correctTokenOutMint,
                    state: testHelper.statePda,
                    boss,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc()
        ).rejects.toThrow("Invalid token in mint");
    });

    test("Take redemption offer should fail when offer expired", async () => {
        // given - create expired offer
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint = testHelper.createMint(boss, BigInt(0), 9);
        
        testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(100e9));
        testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(0));
        testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        testHelper.createTokenAccount(tokenOutMint, vaultAuthorityPda, BigInt(100e9), true);

        const price = new BN("1000000000");
        const currentTime = Math.floor(Date.now() / 1000);
        const startTime = new BN(currentTime - 7200); // 2 hours ago
        const endTime = new BN(currentTime - 3600); // 1 hour ago (expired)

        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const offerId = redemptionOfferAccountData.counter.toNumber();

        // when/then - try to take expired offer
        await expect(
            program.methods
                .takeSingleRedemptionOffer(new BN(offerId), new BN("1000000000"))
                .accounts({
                    tokenInMint,
                    tokenOutMint,
                    state: testHelper.statePda,
                    boss,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc()
        ).rejects.toThrow("Offer has expired");
    });

    test("Take redemption offer should fail when insufficient vault balance", async () => {
        // given - vault has insufficient tokens
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint = testHelper.createMint(boss, BigInt(0), 9);
        
        testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(1000e9));
        testHelper.createTokenAccount(tokenOutMint, user.publicKey, BigInt(0));
        testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        testHelper.createTokenAccount(tokenOutMint, vaultAuthorityPda, BigInt(1e9), true); // Only 1 token in vault

        const price = new BN("1000000000"); // Price = 1.0
        const startTime = new BN(Math.floor(Date.now() / 1000) - 60); // Start 1 minute ago
        const endTime = new BN(startTime.toNumber() + 3600);

        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const offerId = redemptionOfferAccountData.counter.toNumber();

        // when/then - try to take more than vault has (pay 10, get 10, but vault only has 1)
        await expect(
            program.methods
                .takeSingleRedemptionOffer(new BN(offerId), new BN("10000000000"))
                .accounts({
                    tokenInMint,
                    tokenOutMint,
                    state: testHelper.statePda,
                    boss,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc()
        ).rejects.toThrow(); // Should fail due to insufficient vault balance
    });
});