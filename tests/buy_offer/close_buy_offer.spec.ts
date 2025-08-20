import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

describe("Close buy offer", () => {
    let testHelper: TestHelper;
    let program: Program<Onreapp>;
    
    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;
    let boss: PublicKey;
    let buyOfferAccountPda: PublicKey;

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
        
        // Create mints
        tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        tokenOutMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        
        // Initialize program and offers
        await program.methods.initialize().accounts({ boss }).rpc();
        await program.methods.initializeOffers().accounts({ 
            state: testHelper.statePda 
        }).rpc();

        // Get buy offer account PDA
        [buyOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from('buy_offers')], ONREAPP_PROGRAM_ID);
    });

    test("Close buy offer should succeed and remove the offer", async () => {
        // given - create an offer first
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            offerId,
            tokenInMint,
            tokenOutMint,
        });

        // verify offer exists
        let buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        expect(buyOfferAccountData.count.toNumber()).toBe(1);
        expect(buyOfferAccountData.offers[0].offerId.toNumber()).toBe(1);

        // when - close the offer
        await program.methods
            .closeBuyOffer(offerId)
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // then - verify offer is removed
        buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        expect(buyOfferAccountData.count.toNumber()).toBe(0);
        
        // verify the offer slot is cleared
        expect(buyOfferAccountData.offers[0].offerId.toNumber()).toBe(0);
        expect(buyOfferAccountData.offers[0].tokenInMint.toString()).toBe(new PublicKey(0).toString());
        expect(buyOfferAccountData.offers[0].tokenOutMint.toString()).toBe(new PublicKey(0).toString());
    });

    test("Close buy offer should move last offer to removed position", async () => {
        // given - create multiple offers
        const offer1Id = new BN(10);
        const offer2Id = new BN(20);
        const offer3Id = new BN(30);

        await testHelper.makeBuyOffer({
            offerId: offer1Id,
            tokenInMint,
            tokenOutMint,
        });

        const token2In = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const token2Out = testHelper.createMint(boss, BigInt(100_000e9), 9);
        
        await testHelper.makeBuyOffer({
            offerId: offer2Id,
            tokenInMint: token2In,
            tokenOutMint: token2Out,
        });

        const token3In = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const token3Out = testHelper.createMint(boss, BigInt(100_000e9), 9);

        await testHelper.makeBuyOffer({
            offerId: offer3Id,
            tokenInMint: token3In,
            tokenOutMint: token3Out,
        });

        // verify all offers exist
        let buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        expect(buyOfferAccountData.count.toNumber()).toBe(3);

        // when - close the middle offer (index 1)
        await program.methods
            .closeBuyOffer(offer2Id)
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // then - verify the last offer moved to position 1
        buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        expect(buyOfferAccountData.count.toNumber()).toBe(2);
        
        // offer1 should still be at index 0
        expect(buyOfferAccountData.offers[0].offerId.toNumber()).toBe(10);
        
        // offer3 should have moved to index 1 (where offer2 was)
        expect(buyOfferAccountData.offers[1].offerId.toNumber()).toBe(30);
        expect(buyOfferAccountData.offers[1].tokenInMint.toString()).toBe(token3In.toString());
        expect(buyOfferAccountData.offers[1].tokenOutMint.toString()).toBe(token3Out.toString());

        // the last position should be cleared
        expect(buyOfferAccountData.offers[2].offerId.toNumber()).toBe(0);
        expect(buyOfferAccountData.offers[2].tokenInMint.toString()).toBe(new PublicKey(0).toString());
    });

    test("Close last buy offer should clear data and decrease count", async () => {
        // given - clear any existing offers first
        let buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        const currentCount = buyOfferAccountData.count.toNumber();
        
        // Close any existing offers
        for (let i = 0; i < currentCount; i++) {
            const existingOfferId = buyOfferAccountData.offers[i].offerId;
            if (existingOfferId.toNumber() > 0) {
                await program.methods
                    .closeBuyOffer(existingOfferId)
                    .accounts({
                        state: testHelper.statePda,
                    })
                    .rpc();
            }
        }

        // create one offer
        const offerId = new BN(100);
        await testHelper.makeBuyOffer({
            offerId,
            tokenInMint,
            tokenOutMint,
        });

        // verify offer exists
        buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        expect(buyOfferAccountData.count.toNumber()).toBe(1);

        // when - close the only offer
        await program.methods
            .closeBuyOffer(offerId)
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // then - verify count is 0 and data is cleared
        buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        expect(buyOfferAccountData.count.toNumber()).toBe(0);
        
        // verify the offer data is cleared
        expect(buyOfferAccountData.offers[0].offerId.toNumber()).toBe(0);
        expect(buyOfferAccountData.offers[0].tokenInMint.toString()).toBe(new PublicKey(0).toString());
        expect(buyOfferAccountData.offers[0].tokenOutMint.toString()).toBe(new PublicKey(0).toString());
    });

    test("Close buy offer with offer_id 0 should fail", async () => {
        // when/then - try to close with invalid offer_id = 0
        const invalidOfferId = new BN(0);
        await expect(
            program.methods
                .closeBuyOffer(invalidOfferId)
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Offer not found");
    });

    test("Close non-existent buy offer should fail", async () => {
        // when/then - try to close non-existent offer (doesn't matter how many other offers exist)
        const nonExistentOfferId = new BN(999);
        await expect(
            program.methods
                .closeBuyOffer(nonExistentOfferId)
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Offer not found");
    });

    test("Close buy offer should fail when not called by boss", async () => {
        // given - create an offer
        const offerId = new BN(200);
        await testHelper.makeBuyOffer({
            offerId,
            tokenInMint,
            tokenOutMint,
        });

        // when/then - try to close with different signer
        const notBoss = testHelper.createUserAccount();
        
        await expect(
            program.methods
                .closeBuyOffer(offerId)
                .accountsPartial({
                    state: testHelper.statePda,
                    boss: notBoss.publicKey,
                })
                .signers([notBoss])
                .rpc()
        ).rejects.toThrow(); // Should fail due to boss constraint
    });
});