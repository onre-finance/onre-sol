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
});