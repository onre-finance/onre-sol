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

    test("Close buy offer should succeed and clear the offer", async () => {
        // given - create an offer first
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // verify offer exists
        let buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        const initialCounter = buyOfferAccountData.counter.toNumber();
        const createdOffer = buyOfferAccountData.offers.find(offer => offer.offerId.toNumber() === initialCounter);
        expect(createdOffer).toBeDefined();

        // when - close the offer
        await program.methods
            .closeBuyOffer(new BN(initialCounter))
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // then - verify offer is cleared
        buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        
        // Counter should remain the same (not decremented)
        expect(buyOfferAccountData.counter.toNumber()).toBe(initialCounter);
        
        // Find the offer that was closed - should be cleared
        const closedOffer = buyOfferAccountData.offers.find(offer => offer.offerId.toNumber() === initialCounter);
        expect(closedOffer).toBeUndefined(); // Should not exist anymore
        
        // Verify all offers are cleared (since this is the only one)
        const activeOffers = buyOfferAccountData.offers.filter(offer => offer.offerId.toNumber() > 0);
        expect(activeOffers.length).toBe(0);
    });

    test("Close buy offer should clear specific offer without affecting others", async () => {
        // given - create multiple offers
        const initialData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        const startingCounter = initialData.counter.toNumber();

        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const token2In = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const token2Out = testHelper.createMint(boss, BigInt(100_000e9), 9);
        
        await testHelper.makeBuyOffer({
            tokenInMint: token2In,
            tokenOutMint: token2Out,
        });

        const token3In = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const token3Out = testHelper.createMint(boss, BigInt(100_000e9), 9);

        await testHelper.makeBuyOffer({
            tokenInMint: token3In,
            tokenOutMint: token3Out,
        });

        // verify all offers exist
        let buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        expect(buyOfferAccountData.counter.toNumber()).toBe(startingCounter + 3);
        let activeOffers = buyOfferAccountData.offers.filter(offer => offer.offerId.toNumber() > startingCounter);
        expect(activeOffers.length).toBe(3);

        // when - close the middle offer
        const middleOfferId = startingCounter + 2;
        await program.methods
            .closeBuyOffer(new BN(middleOfferId))
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // then - verify only the middle offer is cleared
        buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        
        // Counter remains the same
        expect(buyOfferAccountData.counter.toNumber()).toBe(startingCounter + 3);
        
        // Only 2 active offers remain
        activeOffers = buyOfferAccountData.offers.filter(offer => offer.offerId.toNumber() > startingCounter);
        expect(activeOffers.length).toBe(2);
        
        // First and third offers should still exist
        const firstOffer = buyOfferAccountData.offers.find(offer => offer.offerId.toNumber() === startingCounter + 1);
        const thirdOffer = buyOfferAccountData.offers.find(offer => offer.offerId.toNumber() === startingCounter + 3);
        
        expect(firstOffer).toBeDefined();
        expect(thirdOffer).toBeDefined();
        
        // Middle offer should be cleared
        const middleOffer = buyOfferAccountData.offers.find(offer => offer.offerId.toNumber() === middleOfferId);
        expect(middleOffer).toBeUndefined();
    });

    test("Close offer should clear data properly", async () => {
        // given - create one offer
        const initialData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        const startingCounter = initialData.counter.toNumber();

        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // verify offer exists
        let buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        const newOfferId = startingCounter + 1;
        const createdOffer = buyOfferAccountData.offers.find(offer => offer.offerId.toNumber() === newOfferId);
        expect(createdOffer).toBeDefined();

        // when - close the offer
        await program.methods
            .closeBuyOffer(new BN(newOfferId))
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // then - verify data is cleared
        buyOfferAccountData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        
        // Counter remains unchanged
        expect(buyOfferAccountData.counter.toNumber()).toBe(newOfferId);
        
        // Offer should be cleared (offerId = 0)
        const clearedOffer = buyOfferAccountData.offers.find(offer => offer.offerId.toNumber() === newOfferId);
        expect(clearedOffer).toBeUndefined();
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
        const initialData = await program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        const startingCounter = initialData.counter.toNumber();

        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const newOfferId = startingCounter + 1;

        // when/then - try to close with different signer
        const notBoss = testHelper.createUserAccount();
        
        await expect(
            program.methods
                .closeBuyOffer(new BN(newOfferId))
                .accountsPartial({
                    state: testHelper.statePda,
                    boss: notBoss.publicKey,
                })
                .signers([notBoss])
                .rpc()
        ).rejects.toThrow(); // Should fail due to boss constraint
    });
});