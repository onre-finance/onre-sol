import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

const MAX_REDEMPTION_OFFERS = 50;

describe("Make single redemption offer", () => {
    let testHelper: TestHelper;
    let program: Program<Onreapp>;
    
    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;
    let boss: PublicKey;
    let singleRedemptionOfferAccountPda: PublicKey;

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

        // Get single redemption offer account PDA
        [singleRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from('single_redemption_offers')], ONREAPP_PROGRAM_ID);
    });

    test("Make single redemption offer should succeed", async () => {
        // given
        const startTime = new BN(Math.floor(Date.now() / 1000));
        const endTime = new BN(startTime.toNumber() + 3600); // 1 hour later
        const price = new BN(1000);

        // when
        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        // then
        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        
        expect(redemptionOfferAccountData.counter.toNumber()).toBe(1);
        
        const firstOffer = redemptionOfferAccountData.offers[0];
        expect(firstOffer.offerId.toNumber()).toBe(1);
        expect(firstOffer.tokenInMint.toString()).toBe(tokenInMint.toString());
        expect(firstOffer.tokenOutMint.toString()).toBe(tokenOutMint.toString());
        expect(firstOffer.startTime.toNumber()).toBe(startTime.toNumber());
        expect(firstOffer.endTime.toNumber()).toBe(endTime.toNumber());
        expect(firstOffer.price.toNumber()).toBe(price.toNumber());
    });

    test("Make multiple redemption offers should succeed", async () => {
        // given
        const initialData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const initialCounter = initialData.counter.toNumber();

        // when - create first offer
        const startTime1 = new BN(Math.floor(Date.now() / 1000));
        const endTime1 = new BN(startTime1.toNumber() + 1800); // 30 minutes
        const price1 = new BN(2000);

        await program.methods
            .makeSingleRedemptionOffer(startTime1, endTime1, price1)
            .accounts({
                tokenInMint,
                tokenOutMint,
                state: testHelper.statePda,
            })
            .rpc();

        // create second offer with different tokens
        const token2In = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const token2Out = testHelper.createMint(boss, BigInt(100_000e9), 9);
        
        const startTime2 = new BN(Math.floor(Date.now() / 1000) + 3600); // 1 hour from now
        const endTime2 = new BN(startTime2.toNumber() + 7200); // 2 hours duration
        const price2 = new BN(3000);

        await program.methods
            .makeSingleRedemptionOffer(startTime2, endTime2, price2)
            .accounts({
                tokenInMint: token2In,
                tokenOutMint: token2Out,
                state: testHelper.statePda,
            })
            .rpc();

        // then
        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        
        expect(redemptionOfferAccountData.counter.toNumber()).toBe(initialCounter + 2);

        // Find offers by their auto-generated IDs
        const firstOffer = redemptionOfferAccountData.offers.find(offer => 
            offer.tokenInMint.toString() === tokenInMint.toString() && 
            offer.offerId.toNumber() > initialCounter
        );
        expect(firstOffer).toBeDefined();
        expect(firstOffer!.offerId.toNumber()).toBe(initialCounter + 1);
        expect(firstOffer!.price.toNumber()).toBe(price1.toNumber());
        expect(firstOffer!.startTime.toNumber()).toBe(startTime1.toNumber());

        const secondOffer = redemptionOfferAccountData.offers.find(offer => 
            offer.tokenInMint.toString() === token2In.toString()
        );
        expect(secondOffer).toBeDefined();
        expect(secondOffer!.offerId.toNumber()).toBe(initialCounter + 2);
        expect(secondOffer!.price.toNumber()).toBe(price2.toNumber());
        expect(secondOffer!.startTime.toNumber()).toBe(startTime2.toNumber());
    });

    test("Make redemption offer with invalid token mints should fail", async () => {
        // when/then
        const startTime = new BN(Math.floor(Date.now() / 1000));
        const endTime = new BN(startTime.toNumber() + 3600);
        const price = new BN(1000);

        await expect(
            program.methods
                .makeSingleRedemptionOffer(startTime, endTime, price)
                .accounts({
                    tokenInMint: new PublicKey(0),
                    tokenOutMint: new PublicKey(0),
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow();
    });

    test("Make redemption offer with zero price should succeed", async () => {
        // given - zero price should be allowed (free redemption)
        const startTime = new BN(Math.floor(Date.now() / 1000));
        const endTime = new BN(startTime.toNumber() + 3600);
        const price = new BN(0);

        const uniqueTokenIn = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const uniqueTokenOut = testHelper.createMint(boss, BigInt(100_000e9), 9);

        // when
        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint: uniqueTokenIn,
                tokenOutMint: uniqueTokenOut,
                state: testHelper.statePda,
            })
            .rpc();

        // then
        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        
        const zeroOffer = redemptionOfferAccountData.offers.find(offer => 
            offer.tokenInMint.toString() === uniqueTokenIn.toString()
        );
        expect(zeroOffer).toBeDefined();
        expect(zeroOffer!.price.toNumber()).toBe(0);
    });

    test("Make redemption offer with past start time should succeed", async () => {
        // given - past start time should be allowed (immediately active)
        const startTime = new BN(Math.floor(Date.now() / 1000) - 3600); // 1 hour ago
        const endTime = new BN(Math.floor(Date.now() / 1000) + 3600); // 1 hour from now
        const price = new BN(500);

        const uniqueTokenIn = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const uniqueTokenOut = testHelper.createMint(boss, BigInt(100_000e9), 9);

        // when
        await program.methods
            .makeSingleRedemptionOffer(startTime, endTime, price)
            .accounts({
                tokenInMint: uniqueTokenIn,
                tokenOutMint: uniqueTokenOut,
                state: testHelper.statePda,
            })
            .rpc();

        // then
        const redemptionOfferAccountData = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        
        const pastOffer = redemptionOfferAccountData.offers.find(offer => 
            offer.tokenInMint.toString() === uniqueTokenIn.toString()
        );
        expect(pastOffer).toBeDefined();
        expect(pastOffer!.startTime.toNumber()).toBe(startTime.toNumber());
        expect(pastOffer!.endTime.toNumber()).toBe(endTime.toNumber());
    });

    test("Make more than max redemption offers should fail", async () => {
        // given - check how many offers already exist
        let redemptionOfferAccount = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const existingOffers = redemptionOfferAccount.offers.filter(offer => offer.offerId.toNumber() > 0).length;
        
        console.log(`Existing redemption offers: ${existingOffers}`);
        
        // Fill up remaining slots
        const offersToMake = MAX_REDEMPTION_OFFERS - existingOffers;
        console.log(`Need to make ${offersToMake} more offers`);
        
        const baseTime = Math.floor(Date.now() / 1000);
        
        for (let i = 0; i < offersToMake; i++) {
            console.log(`Making redemption offer ${i + 1}/${offersToMake}`);
            
            // Create unique mints for each offer to avoid duplicate transaction issues
            const uniqueTokenIn = testHelper.createMint(boss, BigInt(100_000e9), 9);
            const uniqueTokenOut = testHelper.createMint(boss, BigInt(100_000e9), 9);
            
            const startTime = new BN(baseTime + i * 100); // Stagger start times
            const endTime = new BN(baseTime + i * 100 + 3600);
            const price = new BN(1000 + i); // Different prices

            await program.methods
                .makeSingleRedemptionOffer(startTime, endTime, price)
                .accounts({
                    tokenInMint: uniqueTokenIn,
                    tokenOutMint: uniqueTokenOut,
                    state: testHelper.statePda,
                })
                .rpc();
        }

        // Verify array is full
        redemptionOfferAccount = await program.account.singleRedemptionOfferAccount.fetch(singleRedemptionOfferAccountPda);
        const activeOffers = redemptionOfferAccount.offers.filter(offer => offer.offerId.toNumber() > 0).length;
        console.log(`Final active redemption offers: ${activeOffers}`);
        expect(activeOffers).toBe(MAX_REDEMPTION_OFFERS);

        // when - try to make one more offer (should fail)
        console.log("Attempting to make one more redemption offer (should fail)");
        
        const finalTokenIn = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const finalTokenOut = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const finalStartTime = new BN(baseTime + 10000);
        const finalEndTime = new BN(baseTime + 13600);
        const finalPrice = new BN(9999);

        await expect(
            program.methods
                .makeSingleRedemptionOffer(finalStartTime, finalEndTime, finalPrice)
                .accounts({
                    tokenInMint: finalTokenIn,
                    tokenOutMint: finalTokenOut,
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Single redemption offer account is full");
    });

    test("Make redemption offer should fail when not called by boss", async () => {
        // given
        const startTime = new BN(Math.floor(Date.now() / 1000));
        const endTime = new BN(startTime.toNumber() + 3600);
        const price = new BN(1000);

        const uniqueTokenIn = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const uniqueTokenOut = testHelper.createMint(boss, BigInt(100_000e9), 9);

        // when/then - try to create with different signer
        const notBoss = testHelper.createUserAccount();
        
        await expect(
            program.methods
                .makeSingleRedemptionOffer(startTime, endTime, price)
                .accountsPartial({
                    tokenInMint: uniqueTokenIn,
                    tokenOutMint: uniqueTokenOut,
                    state: testHelper.statePda,
                    boss: notBoss.publicKey,
                })
                .signers([notBoss])
                .rpc()
        ).rejects.toThrow(); // Should fail due to boss constraint
    });
});