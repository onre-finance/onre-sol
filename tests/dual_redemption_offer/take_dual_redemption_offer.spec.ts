import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

describe("Take dual redemption offer", () => {
    let testHelper: TestHelper;
    let program: Program<Onreapp>;
    
    let boss: PublicKey;
    let user: any;
    let dualRedemptionOfferAccountPda: PublicKey;
    let vaultAuthorityPda: PublicKey;

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
        [dualRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from('dual_redemption_offers')], ONREAPP_PROGRAM_ID);
        [vaultAuthorityPda] = PublicKey.findProgramAddressSync([Buffer.from('vault_authority')], ONREAPP_PROGRAM_ID);
    });

    test("Take dual redemption offer with 80/20 ratio should succeed", async () => {
        // given - create tokens and set up vault
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint1 = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint2 = testHelper.createMint(boss, BigInt(0), 6);
        
        // Create user accounts and fund them
        const userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(1000e9)); // 1000 tokens
        const userTokenOutAccount1 = testHelper.createTokenAccount(tokenOutMint1, user.publicKey, BigInt(0));
        const userTokenOutAccount2 = testHelper.createTokenAccount(tokenOutMint2, user.publicKey, BigInt(0));
        const bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        
        // Set up vault with tokens
        testHelper.createTokenAccount(tokenOutMint1, boss, BigInt(1000e9)); // 1000 tokens for boss
        testHelper.createTokenAccount(tokenOutMint2, boss, BigInt(1000e6)); // 1000 tokens for boss
        const vaultTokenOutAccount1 = getAssociatedTokenAddressSync(tokenOutMint1, vaultAuthorityPda, true);
        const vaultTokenOutAccount2 = getAssociatedTokenAddressSync(tokenOutMint2, vaultAuthorityPda, true);
        
        // Deposit tokens to vault
        await program.methods
            .vaultDeposit(new BN("1000000000000")) // 1000 tokens with 9 decimals
            .accounts({
                tokenMint: tokenOutMint1,
                state: testHelper.statePda,
            })
            .rpc();

        await program.methods
            .vaultDeposit(new BN("1000000000")) // 1000 tokens with 6 decimals
            .accounts({
                tokenMint: tokenOutMint2,
                state: testHelper.statePda,
            })
            .rpc();

        // Create dual redemption offer: price1 = 2.0, price2 = 1.0, ratio = 8000 (80% for token1, 20% for token2)
        const price1 = new BN("2000000000"); // 2.0 * 10^9
        const price2 = new BN("1000000000"); // 1.0 * 10^9
        const ratioBasisPoints = new BN(8000); // 80% for token_out_1
        const startTime = new BN(Math.floor(Date.now() / 1000) - 60); // Start 1 minute ago
        const endTime = new BN(startTime.toNumber() + 3600);

        await program.methods
            .makeDualRedemptionOffer(startTime, endTime, price1, price2, ratioBasisPoints)
            .accounts({
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2,
                state: testHelper.statePda,
            })
            .rpc();

        // Get offer ID
        const dualRedemptionOfferAccountData = await program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
        const offerId = dualRedemptionOfferAccountData.counter.toNumber();

        // when - user takes offer: pays 100 token_in
        // Expected calculation:
        // - 80% of 100 = 80 tokens go to token_out_1 at price 2.0 → 80/2 = 40 token_out_1
        // - 20% of 100 = 20 tokens go to token_out_2 at price 1.0 → 20/1 = 20 token_out_2
        const tokenInAmount = new BN("100000000000"); // 100 tokens with 9 decimals

        await program.methods
            .takeDualRedemptionOffer(new BN(offerId), tokenInAmount)
            .accountsPartial({
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2,
                state: testHelper.statePda,
                boss,
                user: user.publicKey,
            })
            .signers([user])
            .rpc();

        // then - verify balances
        // User should have: 1000 - 100 = 900 token_in, 0 + 40 = 40 token_out_1, 0 + 20 = 20 token_out_2
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt("900000000000"));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount1, BigInt("40000000000"));
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount2, BigInt("20000000"));
        
        // Boss should have: 0 + 100 = 100 token_in
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt("100000000000"));
        
        // Vault should have: 1000 - 40 = 960 token_out_1, 1000 - 20 = 980 token_out_2
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount1, BigInt("960000000000"));
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount2, BigInt("980000000"));
    });

    test("Take dual redemption offer with 0/100 ratio (all to token2) should succeed", async () => {
        // given - create tokens and set up vault
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9); // Standard 9 decimals
        const tokenOutMint1 = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint2 = testHelper.createMint(boss, BigInt(0), 9);
        
        // Create user accounts and fund them
        const userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(1000000000000)); // 1000 tokens with 9 decimals
        const userTokenOutAccount1 = testHelper.createTokenAccount(tokenOutMint1, user.publicKey, BigInt(0));
        const userTokenOutAccount2 = testHelper.createTokenAccount(tokenOutMint2, user.publicKey, BigInt(0));
        const bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        
        // Set up vault with tokens
        testHelper.createTokenAccount(tokenOutMint1, boss, BigInt("1000000000000")); // 1000 tokens with 9 decimals
        testHelper.createTokenAccount(tokenOutMint2, boss, BigInt("1000000000000")); // 1000 tokens with 9 decimals
        const vaultTokenOutAccount1 = getAssociatedTokenAddressSync(tokenOutMint1, vaultAuthorityPda, true);
        const vaultTokenOutAccount2 = getAssociatedTokenAddressSync(tokenOutMint2, vaultAuthorityPda, true);
        
        // Deposit tokens to vault
        await program.methods
            .vaultDeposit(new BN("1000000000000")) // 1000 tokens
            .accounts({
                tokenMint: tokenOutMint1,
                state: testHelper.statePda,
            })
            .rpc();

        await program.methods
            .vaultDeposit(new BN("1000000000000")) // 1000 tokens with 9 decimals
            .accounts({
                tokenMint: tokenOutMint2,
                state: testHelper.statePda,
            })
            .rpc();

        // Create dual redemption offer: price1 = 3.0, price2 = 1.5, ratio = 0 (0% for token1, 100% for token2)
        const price1 = new BN("3000000000"); // 3.0 * 10^9
        const price2 = new BN("1500000000"); // 1.5 * 10^9
        const ratioBasisPoints = new BN(0); // 0% for token_out_1, 100% for token_out_2
        const startTime = new BN(Math.floor(Date.now() / 1000) - 60);
        const endTime = new BN(startTime.toNumber() + 3600);

        await program.methods
            .makeDualRedemptionOffer(startTime, endTime, price1, price2, ratioBasisPoints)
            .accounts({
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2,
                state: testHelper.statePda,
            })
            .rpc();

        // Get offer ID
        const dualRedemptionOfferAccountData = await program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
        const offerId = dualRedemptionOfferAccountData.counter.toNumber();

        // when - user takes offer: pays 30 token_in (9 decimals)
        // Expected calculation:
        // - 0% of 30 = 0 tokens go to token_out_1 → 0 token_out_1
        // - 100% of 30 = 30 tokens go to token_out_2 at price 1.5 → 30/1.5 = 20 token_out_2
        const tokenInAmount = new BN("30000000000"); // 30 tokens with 9 decimals

        await program.methods
            .takeDualRedemptionOffer(new BN(offerId), tokenInAmount)
            .accountsPartial({
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2,
                state: testHelper.statePda,
                boss,
                user: user.publicKey,
            })
            .signers([user])
            .rpc();

        // then - verify balances
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt("970000000000")); // 1000 - 30 = 970 (9 decimals)
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount1, BigInt("0")); // Should get 0
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount2, BigInt("20000000000")); // Should get 20 tokens (9 decimals)
        
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt("30000000000")); // Should get 30 (9 decimals)
        
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount1, BigInt("1000000000000")); // Should remain 1000
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount2, BigInt("980000000000")); // 1000 - 20 = 980 (9 decimals)
    });

    test("Take dual redemption offer with 100/0 ratio (all to token1) should succeed", async () => {
        // given - create tokens and set up vault
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint1 = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint2 = testHelper.createMint(boss, BigInt(0), 9);
        
        // Create user accounts and fund them
        const userTokenInAccount = testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(500000000000)); // 500 tokens with 9 decimals
        const userTokenOutAccount1 = testHelper.createTokenAccount(tokenOutMint1, user.publicKey, BigInt(0));
        const userTokenOutAccount2 = testHelper.createTokenAccount(tokenOutMint2, user.publicKey, BigInt(0));
        const bossTokenInAccount = testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        
        // Set up vault with tokens
        testHelper.createTokenAccount(tokenOutMint1, boss, BigInt(1000000000000)); // 1000 tokens with 9 decimals
        testHelper.createTokenAccount(tokenOutMint2, boss, BigInt(1000000000000)); // 1000 tokens with 9 decimals
        const vaultTokenOutAccount1 = getAssociatedTokenAddressSync(tokenOutMint1, vaultAuthorityPda, true);
        const vaultTokenOutAccount2 = getAssociatedTokenAddressSync(tokenOutMint2, vaultAuthorityPda, true);
        
        // Deposit tokens to vault
        await program.methods
            .vaultDeposit(new BN("1000000000000")) // 1000 tokens
            .accounts({
                tokenMint: tokenOutMint1,
                state: testHelper.statePda,
            })
            .rpc();

        await program.methods
            .vaultDeposit(new BN("1000000000000")) // 1000 tokens with 9 decimals
            .accounts({
                tokenMint: tokenOutMint2,
                state: testHelper.statePda,
            })
            .rpc();

        // Create dual redemption offer: price1 = 0.5, price2 = 2.0, ratio = 10000 (100% for token1, 0% for token2)
        const price1 = new BN("500000000"); // 0.5 * 10^9
        const price2 = new BN("2000000000"); // 2.0 * 10^9
        const ratioBasisPoints = new BN(10000); // 100% for token_out_1, 0% for token_out_2
        const startTime = new BN(Math.floor(Date.now() / 1000) - 60);
        const endTime = new BN(startTime.toNumber() + 3600);

        await program.methods
            .makeDualRedemptionOffer(startTime, endTime, price1, price2, ratioBasisPoints)
            .accounts({
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2,
                state: testHelper.statePda,
            })
            .rpc();

        // Get offer ID
        const dualRedemptionOfferAccountData = await program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
        const offerId = dualRedemptionOfferAccountData.counter.toNumber();

        // when - user takes offer: pays 10 token_in (9 decimals)
        // Expected calculation:
        // - 100% of 10 = 10 tokens go to token_out_1 at price 0.5 → 10/0.5 = 20 token_out_1
        // - 0% of 10 = 0 tokens go to token_out_2 → 0 token_out_2
        const tokenInAmount = new BN("10000000000"); // 10 tokens with 9 decimals

        await program.methods
            .takeDualRedemptionOffer(new BN(offerId), tokenInAmount)
            .accountsPartial({
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2,
                state: testHelper.statePda,
                boss,
                user: user.publicKey,
            })
            .signers([user])
            .rpc();

        // then - verify balances
        await testHelper.expectTokenAccountAmountToBe(userTokenInAccount, BigInt("490000000000")); // 500 - 10
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount1, BigInt("20000000000")); // Should get 20
        await testHelper.expectTokenAccountAmountToBe(userTokenOutAccount2, BigInt("0")); // Should get 0
        
        await testHelper.expectTokenAccountAmountToBe(bossTokenInAccount, BigInt("10000000000")); // Should get 10
        
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount1, BigInt("980000000000")); // 1000 - 20 = 980
        await testHelper.expectTokenAccountAmountToBe(vaultTokenOutAccount2, BigInt("1000000000000")); // Should remain 1000
    });

    test("Take dual redemption offer should fail when offer doesn't exist", async () => {
        // given
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint1 = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint2 = testHelper.createMint(boss, BigInt(0), 6);
        
        testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(100000000000)); // 100 tokens with 9 decimals
        testHelper.createTokenAccount(tokenOutMint1, user.publicKey, BigInt(0));
        testHelper.createTokenAccount(tokenOutMint2, user.publicKey, BigInt(0));
        testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        
        // Set up vault with tokens to avoid AccountNotInitialized
        testHelper.createTokenAccount(tokenOutMint1, boss, BigInt(1000000000000)); // 1000 tokens
        testHelper.createTokenAccount(tokenOutMint2, boss, BigInt(1000000000)); // 1000 tokens
        await program.methods
            .vaultDeposit(new BN("1000000000000"))
            .accounts({
                tokenMint: tokenOutMint1,
                state: testHelper.statePda,
            })
            .rpc();
        await program.methods
            .vaultDeposit(new BN("1000000000"))
            .accounts({
                tokenMint: tokenOutMint2,
                state: testHelper.statePda,
            })
            .rpc();

        // when/then - try to take non-existent offer
        await expect(
            program.methods
                .takeDualRedemptionOffer(new BN(9999), new BN("1000000000"))
                .accountsPartial({
                    tokenInMint,
                    tokenOutMint1,
                    tokenOutMint2,
                    state: testHelper.statePda,
                    boss,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc()
        ).rejects.toThrow("Offer not found");
    });

    test("Take dual redemption offer should fail when offer expired", async () => {
        // given - create expired offer
        const tokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint1 = testHelper.createMint(boss, BigInt(0), 9);
        const tokenOutMint2 = testHelper.createMint(boss, BigInt(0), 6);
        
        testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(100000000000)); // 100 tokens with 9 decimals
        testHelper.createTokenAccount(tokenOutMint1, user.publicKey, BigInt(0));
        testHelper.createTokenAccount(tokenOutMint2, user.publicKey, BigInt(0));
        testHelper.createTokenAccount(tokenInMint, boss, BigInt(0));
        
        // Set up vault with tokens to avoid AccountNotInitialized
        testHelper.createTokenAccount(tokenOutMint1, boss, BigInt(1000000000000)); // 1000 tokens
        testHelper.createTokenAccount(tokenOutMint2, boss, BigInt(1000000000)); // 1000 tokens
        await program.methods
            .vaultDeposit(new BN("1000000000000"))
            .accounts({
                tokenMint: tokenOutMint1,
                state: testHelper.statePda,
            })
            .rpc();
        await program.methods
            .vaultDeposit(new BN("1000000000"))
            .accounts({
                tokenMint: tokenOutMint2,
                state: testHelper.statePda,
            })
            .rpc();

        const price1 = new BN("1000000000");
        const price2 = new BN("500000000");
        const currentTime = Math.floor(Date.now() / 1000);
        const startTime = new BN(currentTime - 7200); // 2 hours ago
        const endTime = new BN(currentTime - 3600); // 1 hour ago (expired)

        await program.methods
            .makeDualRedemptionOffer(startTime, endTime, price1, price2, new BN(5000))
            .accounts({
                tokenInMint,
                tokenOutMint1,
                tokenOutMint2,
                state: testHelper.statePda,
            })
            .rpc();

        const dualRedemptionOfferAccountData = await program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
        const offerId = dualRedemptionOfferAccountData.counter.toNumber();

        // when/then - try to take expired offer
        await expect(
            program.methods
                .takeDualRedemptionOffer(new BN(offerId), new BN("1000000000"))
                .accountsPartial({
                    tokenInMint,
                    tokenOutMint1,
                    tokenOutMint2,
                    state: testHelper.statePda,
                    boss,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc()
        ).rejects.toThrow("Offer has expired");
    });

    test("Take dual redemption offer should fail with wrong token mints", async () => {
        // given - create offer with specific mints
        const correctTokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        const correctTokenOutMint1 = testHelper.createMint(boss, BigInt(0), 9);
        const correctTokenOutMint2 = testHelper.createMint(boss, BigInt(0), 6);
        const wrongTokenInMint = testHelper.createMint(boss, BigInt(0), 9);
        
        testHelper.createTokenAccount(wrongTokenInMint, user.publicKey, BigInt(100000000000)); // 100 tokens with 9 decimals
        testHelper.createTokenAccount(correctTokenOutMint1, user.publicKey, BigInt(0));
        testHelper.createTokenAccount(correctTokenOutMint2, user.publicKey, BigInt(0));
        testHelper.createTokenAccount(wrongTokenInMint, boss, BigInt(0));
        
        // Set up vault with tokens to avoid AccountNotInitialized
        testHelper.createTokenAccount(correctTokenOutMint1, boss, BigInt(1000000000000)); // 1000 tokens
        testHelper.createTokenAccount(correctTokenOutMint2, boss, BigInt(1000000000)); // 1000 tokens
        await program.methods
            .vaultDeposit(new BN("1000000000000"))
            .accounts({
                tokenMint: correctTokenOutMint1,
                state: testHelper.statePda,
            })
            .rpc();
        await program.methods
            .vaultDeposit(new BN("1000000000"))
            .accounts({
                tokenMint: correctTokenOutMint2,
                state: testHelper.statePda,
            })
            .rpc();

        const price1 = new BN("1000000000");
        const price2 = new BN("500000000");
        const startTime = new BN(Math.floor(Date.now() / 1000) - 60);
        const endTime = new BN(startTime.toNumber() + 3600);

        await program.methods
            .makeDualRedemptionOffer(startTime, endTime, price1, price2, new BN(5000))
            .accounts({
                tokenInMint: correctTokenInMint,
                tokenOutMint1: correctTokenOutMint1,
                tokenOutMint2: correctTokenOutMint2,
                state: testHelper.statePda,
            })
            .rpc();

        const dualRedemptionOfferAccountData = await program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
        const offerId = dualRedemptionOfferAccountData.counter.toNumber();

        // when/then - try to take offer with wrong token_in mint
        await expect(
            program.methods
                .takeDualRedemptionOffer(new BN(offerId), new BN("1000000000"))
                .accountsPartial({
                    tokenInMint: wrongTokenInMint,
                    tokenOutMint1: correctTokenOutMint1,
                    tokenOutMint2: correctTokenOutMint2,
                    state: testHelper.statePda,
                    boss,
                    user: user.publicKey,
                })
                .signers([user])
                .rpc()
        ).rejects.toThrow("Invalid token in mint");
    });
});