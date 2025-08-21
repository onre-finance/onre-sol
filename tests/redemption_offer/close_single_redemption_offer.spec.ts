import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

describe("Close single redemption offer", () => {
    let testHelper: TestHelper;
    let program: Program<Onreapp>;
    
    let boss: PublicKey;
    let singleRedemptionOfferAccountPda: PublicKey;

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
        
        // Initialize program and offers
        await program.methods.initialize().accounts({ boss }).rpc();
        await program.methods.initializeOffers().accounts({ 
            state: testHelper.statePda 
        }).rpc();

        // Get single redemption offer account PDA
        [singleRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from('single_redemption_offers')], ONREAPP_PROGRAM_ID);
    });

    test("Close single redemption offer should succeed", async () => {
        // given - create a redemption offer first
        const tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const tokenOutMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const startTime = new BN(Math.floor(Date.now() / 1000));
        const endTime = new BN(startTime.toNumber() + 3600);
        const price = new BN(1000);

        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        // Verify offer was created
        let redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const offerId = redemptionOfferAccountData.counter.toNumber();
        
        const createdOffer = redemptionOfferAccountData.offers.find(offer => offer.offerId.toNumber() === offerId);
        expect(createdOffer).toBeDefined();
        expect(createdOffer!.offerId.toNumber()).toBe(offerId);
        
        // Count active offers before closing
        const activeOffersBefore = redemptionOfferAccountData.offers.filter(offer => offer.offerId.toNumber() > 0).length;

        // when - close the offer
        await program.methods
            .closeSingleRedemptionOffer(new BN(offerId))
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // then - verify offer is cleared
        redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        
        // The offer with the original ID should no longer exist
        const closedOffer = redemptionOfferAccountData.offers.find(offer => offer.offerId.toNumber() === offerId);
        expect(closedOffer).toBeUndefined();
        
        // Count active offers after closing - should be one less
        const activeOffersAfter = redemptionOfferAccountData.offers.filter(offer => offer.offerId.toNumber() > 0).length;
        expect(activeOffersAfter).toBe(activeOffersBefore - 1);
        
        // Counter should remain unchanged (we don't decrease it when closing)
        expect(redemptionOfferAccountData.counter.toNumber()).toBe(offerId);
    });

    test("Close non-existent redemption offer should fail", async () => {
        // when/then - try to close offer that doesn't exist
        const nonExistentOfferId = new BN(9999);
        
        await expect(
            program.methods
                .closeSingleRedemptionOffer(nonExistentOfferId)
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Offer not found");
    });

    test("Close redemption offer with zero ID should fail", async () => {
        // when/then - try to close offer with ID 0
        await expect(
            program.methods
                .closeSingleRedemptionOffer(new BN(0))
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Offer not found");
    });

    test("Close redemption offer should fail when not called by boss", async () => {
        // given - create a redemption offer first
        const tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const tokenOutMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const startTime = new BN(Math.floor(Date.now() / 1000));
        const endTime = new BN(startTime.toNumber() + 3600);
        const price = new BN(1500);

        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        // Get the offer ID
        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const offerId = redemptionOfferAccountData.counter.toNumber();

        // when/then - try to close with different signer
        const notBoss = testHelper.createUserAccount();
        
        await expect(
            program.methods
                .closeSingleRedemptionOffer(new BN(offerId))
                .accountsPartial({
                    state: testHelper.statePda,
                    boss: notBoss.publicKey,
                })
                .signers([notBoss])
                .rpc()
        ).rejects.toThrow(); // Should fail due to boss constraint
    });

    test("Close multiple redemption offers should succeed", async () => {
        // given - create multiple redemption offers
        const offers = [];
        const baseTime = Math.floor(Date.now() / 1000);
        
        for (let i = 0; i < 3; i++) {
            const tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
            const tokenOutMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
            const startTime = new BN(baseTime + i * 100);
            const endTime = new BN(baseTime + i * 100 + 3600);
            const price = new BN(1000 + i * 100);

            await program.methods
                .makeSingleRedemptionOffer(startTime, endTime, price)
                .accounts({
                    tokenInMint,
                    tokenOutMint,
                    state: testHelper.statePda,
                })
                .rpc();

            offers.push({ tokenInMint, tokenOutMint, price: price.toNumber() });
        }

        // Get all offer IDs
        let redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const finalCounter = redemptionOfferAccountData.counter.toNumber();
        const offerIds = [finalCounter - 2, finalCounter - 1, finalCounter]; // Last 3 offers

        // when - close the first and third offers
        await program.methods
            .closeSingleRedemptionOffer(new BN(offerIds[0]))
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        await program.methods
            .closeSingleRedemptionOffer(new BN(offerIds[2]))
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // then - verify correct offers are closed
        redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        
        // First offer should be closed (cleared)
        const firstOffer = redemptionOfferAccountData.offers.find(offer => offer.offerId.toNumber() === offerIds[0]);
        expect(firstOffer).toBeUndefined();

        // Second offer should still exist
        const secondOffer = redemptionOfferAccountData.offers.find(offer => offer.offerId.toNumber() === offerIds[1]);
        expect(secondOffer).toBeDefined();
        expect(secondOffer!.price.toNumber()).toBe(offers[1].price);

        // Third offer should be closed (cleared)
        const thirdOffer = redemptionOfferAccountData.offers.find(offer => offer.offerId.toNumber() === offerIds[2]);
        expect(thirdOffer).toBeUndefined();
    });
});