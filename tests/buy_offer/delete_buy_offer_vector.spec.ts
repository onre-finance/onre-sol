import {PublicKey} from "@solana/web3.js";
import {ONREAPP_PROGRAM_ID, TestHelper} from "../test_helper";
import {AddedProgram, startAnchor} from "solana-bankrun";
import {Onreapp} from "../../target/types/onreapp";
import {BankrunProvider} from "anchor-bankrun";
import {BN, Program} from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

describe("Delete Buy Offer Vector", () => {
    let testHelper: TestHelper;
    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;
    let boss: PublicKey;

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

        // Create mints
        tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        tokenOutMint = testHelper.createMint(boss, BigInt(100_000e9), 9);

        // Initialize program and offers
        await program.methods.initialize().accounts({boss}).rpc();
        await program.methods.initializeOffers().accounts({
            state: testHelper.statePda
        }).rpc();
    });

    it("Should delete an existing vector from a buy offer", async () => {
        const offerId = new BN(1);

        // Create a buy offer
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add a vector to the offer
        await testHelper.program.methods
            .addBuyOfferVector(
                offerId,
                new BN(currentTime + 1000),
                new BN(1000000),
                new BN(5000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify vector was added
        const [buyOffersPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("buy_offers")],
            ONREAPP_PROGRAM_ID
        );

        let buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        let offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        let activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);
        expect(activeVectors.length).toBe(1);
        expect(activeVectors[0].vectorId.toNumber()).toBe(1);

        // Delete the vector
        await testHelper.program.methods
            .deleteBuyOfferVector(offerId, new BN(1))
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify vector was deleted
        buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);
        expect(activeVectors.length).toBe(0);
    });

    it("Should fail when offer_id is zero", async () => {
        await expect(
            testHelper.program.methods
                .deleteBuyOfferVector(new BN(0), new BN(1))
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Buy offer with the specified ID was not found");
    });

    it("Should fail when vector_id is zero", async () => {
        const offerId = new BN(1);

        // Create a buy offer
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        await expect(
            testHelper.program.methods
                .deleteBuyOfferVector(offerId, new BN(0))
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Vector with the specified ID was not found in the offer");
    });

    it("Should fail when offer doesn't exist", async () => {
        const nonExistentOfferId = new BN(999);

        await expect(
            testHelper.program.methods
                .deleteBuyOfferVector(nonExistentOfferId, new BN(1))
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Offer not found");
    });

    it("Should fail when vector doesn't exist in the offer", async () => {
        const offerId = new BN(1);

        // Create a buy offer
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        await expect(
            testHelper.program.methods
                .deleteBuyOfferVector(offerId, new BN(999))
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Vector with the specified ID was not found in the offer");
    });

    it("Should delete specific vector while keeping others", async () => {
        const offerId = new BN(1);

        // Create a buy offer
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add three vectors
        for (let i = 1; i <= 3; i++) {
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + (i * 1000)),
                    new BN(i * 1000000),
                    new BN(5000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();
        }

        // Verify all vectors were added
        const [buyOffersPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("buy_offers")],
            ONREAPP_PROGRAM_ID
        );

        let buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        let offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        let activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);
        expect(activeVectors.length).toBe(3);

        // Delete the middle vector (vector_id = 2)
        await testHelper.program.methods
            .deleteBuyOfferVector(offerId, new BN(2))
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify only vector 2 was deleted
        buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

        expect(activeVectors.length).toBe(2);
        const vectorIds = activeVectors.map(v => v.vectorId.toNumber()).sort();
        expect(vectorIds).toEqual([1, 3]);

        // Verify prices of remaining vectors
        const vectorPrices = activeVectors.map(v => v.startPrice.toNumber()).sort();
        expect(vectorPrices).toContain(1000000); // Vector 1
        expect(vectorPrices).toContain(3000000); // Vector 3
        expect(vectorPrices).not.toContain(2000000); // Vector 2 deleted
    });

    it("Should reject when called by non-boss", async () => {
        const offerId = new BN(1);

        // Create a buy offer
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add a vector
        await testHelper.program.methods
            .addBuyOfferVector(
                offerId,
                new BN(currentTime + 1000),
                new BN(1000000),
                new BN(5000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        const notBoss = testHelper.createUserAccount();

        await expect(
            testHelper.program.methods
                .deleteBuyOfferVector(offerId, new BN(1))
                .accountsPartial({
                    state: testHelper.statePda,
                    boss: notBoss.publicKey,
                })
                .signers([notBoss])
                .rpc()
        ).rejects.toThrow(); // Should fail due to boss constraint
    });

    describe("Previously Active Vector Validation", () => {
        it("Should prevent deletion of previously active vector", async () => {
            const offerId = new BN(1);

            // Create a buy offer
            await testHelper.makeBuyOffer({
                tokenInMint,
                tokenOutMint,
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add only 2 vectors in the future to keep it simple
            // Vector 1: will become previous active
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 100), // 100 seconds in future
                    new BN(1000000),
                    new BN(5000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Vector 2: will become currently active
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 200), // 200 seconds in future
                    new BN(2000000),
                    new BN(7500),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Advance time to make vector 2 active (and vector 1 previous active)
            await testHelper.advanceClockBy(250); // Move 250 seconds forward (past both vectors)

            // Verify we have 2 vectors
            const [buyOffersPda] = PublicKey.findProgramAddressSync(
                [Buffer.from("buy_offers")],
                ONREAPP_PROGRAM_ID
            );
            const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
            const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
            const activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(2);

            // Now try to delete vector 1 (previous active) - should fail
            await expect(
                testHelper.program.methods
                    .deleteBuyOfferVector(offerId, new BN(1))
                    .accounts({
                        state: testHelper.statePda,
                    })
                    .rpc()
            ).rejects.toThrow("Cannot delete previously active vector");
        });

        it("Should allow deletion of current active vector", async () => {
            const offerId = new BN(1);

            // Create a buy offer
            await testHelper.makeBuyOffer({
                tokenInMint,
                tokenOutMint,
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add vectors in the future
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 10), // 10 seconds in future
                    new BN(1000000),
                    new BN(5000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 20), // 20 seconds in future
                    new BN(2000000),
                    new BN(7500),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Advance time to make vector 2 active
            await testHelper.advanceClockBy(25);

            // Delete the current active vector (vector_id = 2) - should succeed
            await testHelper.program.methods
                .deleteBuyOfferVector(offerId, new BN(2))
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Verify deletion succeeded
            const [buyOffersPda] = PublicKey.findProgramAddressSync(
                [Buffer.from("buy_offers")],
                ONREAPP_PROGRAM_ID
            );

            const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
            const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
            const activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(1);
            expect(activeVectors[0].vectorId.toNumber()).toBe(1);
        });

        it("Should allow deletion of future vector", async () => {
            const offerId = new BN(1);

            // Create a buy offer
            await testHelper.makeBuyOffer({
                tokenInMint,
                tokenOutMint,
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add vectors in the future
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 10), // 10 seconds in future
                    new BN(1000000),
                    new BN(5000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 20), // 20 seconds in future
                    new BN(2000000),
                    new BN(7500),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 30), // 30 seconds in future
                    new BN(3000000),
                    new BN(10000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Advance time to make vector 2 active (vector 3 remains future)
            await testHelper.advanceClockBy(25);

            // Delete the future vector (vector_id = 3) - should succeed
            await testHelper.program.methods
                .deleteBuyOfferVector(offerId, new BN(3))
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Verify deletion succeeded
            const [buyOffersPda] = PublicKey.findProgramAddressSync(
                [Buffer.from("buy_offers")],
                ONREAPP_PROGRAM_ID
            );

            const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
            const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
            const activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(2);
            const vectorIds = activeVectors.map(v => v.vectorId.toNumber()).sort();
            expect(vectorIds).toEqual([1, 2]);
        });

        it("Should allow deletion when there's only one vector (no previous vector)", async () => {
            const offerId = new BN(1);

            // Create a buy offer
            await testHelper.makeBuyOffer({
                tokenInMint,
                tokenOutMint,
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add only one vector in the future
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 10), // 10 seconds in future
                    new BN(1000000),
                    new BN(5000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Advance time to make it active
            await testHelper.advanceClockBy(15);

            // Delete the only vector - should succeed (no previous vector exists)
            await testHelper.program.methods
                .deleteBuyOfferVector(offerId, new BN(1))
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Verify deletion succeeded
            const [buyOffersPda] = PublicKey.findProgramAddressSync(
                [Buffer.from("buy_offers")],
                ONREAPP_PROGRAM_ID
            );

            const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
            const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
            const activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(0);
        });

        it("Should allow deletion of past vectors that are not previously active", async () => {
            const offerId = new BN(1);

            // Create a buy offer
            await testHelper.makeBuyOffer({
                tokenInMint,
                tokenOutMint,
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add 4 vectors in the future to create a sequence
            // Vector 1: will be old past vector (deletable)
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 100), // 100 seconds in future
                    new BN(1000000),
                    new BN(5000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Vector 2: will be old past vector (deletable)
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 200), // 200 seconds in future
                    new BN(2000000),
                    new BN(6000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Vector 3: will become previously active (protected)
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 300), // 300 seconds in future
                    new BN(3000000),
                    new BN(7000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Vector 4: will become currently active
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 400), // 400 seconds in future
                    new BN(4000000),
                    new BN(8000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Advance time to make vector 4 active (vector 3 becomes previous active)
            await testHelper.advanceClockBy(450);

            // Verify we have all 4 vectors
            const [buyOffersPda] = PublicKey.findProgramAddressSync(
                [Buffer.from("buy_offers")],
                ONREAPP_PROGRAM_ID
            );

            let buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
            let offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
            let activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);
            
            expect(activeVectors.length).toBe(4);

            // Try to delete vector 1 (old past vector, not previously active) - should succeed
            await testHelper.program.methods
                .deleteBuyOfferVector(offerId, new BN(1))
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Verify vector 1 was deleted
            buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
            offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
            activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);
            
            expect(activeVectors.length).toBe(3);
            expect(activeVectors.map(v => v.vectorId.toNumber()).sort()).toEqual([2, 3, 4]);

            // Try to delete vector 2 (another old past vector, not previously active) - should succeed
            await testHelper.program.methods
                .deleteBuyOfferVector(offerId, new BN(2))
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Verify vector 2 was deleted
            buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
            offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
            activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);
            
            expect(activeVectors.length).toBe(2);
            expect(activeVectors.map(v => v.vectorId.toNumber()).sort()).toEqual([3, 4]);

            // Try to delete vector 3 (previously active) - should fail
            await expect(
                testHelper.program.methods
                    .deleteBuyOfferVector(offerId, new BN(3))
                    .accounts({
                        state: testHelper.statePda,
                    })
                    .rpc()
            ).rejects.toThrow("Cannot delete previously active vector");
        });

        it("Should allow deletion when all vectors are in the future (no active vector)", async () => {
            const offerId = new BN(1);

            // Create a buy offer
            await testHelper.makeBuyOffer({
                tokenInMint,
                tokenOutMint,
            });

            const currentTime = await testHelper.getCurrentClockTime();

            // Add vectors that are all in the future
            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 100), // 100 seconds in future
                    new BN(1000000),
                    new BN(5000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 200), // 200 seconds in future
                    new BN(2000000),
                    new BN(7500),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Delete the first future vector - should succeed (no active vector means no previous vector)
            await testHelper.program.methods
                .deleteBuyOfferVector(offerId, new BN(1))
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            // Verify deletion succeeded
            const [buyOffersPda] = PublicKey.findProgramAddressSync(
                [Buffer.from("buy_offers")],
                ONREAPP_PROGRAM_ID
            );

            const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
            const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
            const activeVectors = offer.vectors.filter(v => v.vectorId.toNumber() !== 0);

            expect(activeVectors.length).toBe(1);
            expect(activeVectors[0].vectorId.toNumber()).toBe(2);
        });
    });
});