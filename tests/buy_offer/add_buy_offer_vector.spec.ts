import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

const MAX_SEGMENTS = 10;

describe("Add Buy Offer Vector", () => {
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
        await program.methods.initialize().accounts({ boss }).rpc();
        await program.methods.initializeOffers().accounts({ 
            state: testHelper.statePda 
        }).rpc();
    });

    it("Should create a buy offer and add a time vector", async () => {
        // First create a buy offer using testHelper
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // Derive the buy offers account PDA
        const [buyOffersPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("buy_offers")],
            ONREAPP_PROGRAM_ID
        );

        // Get the first offer (auto-generated ID)
        const buyOfferAccountBefore = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer = buyOfferAccountBefore.offers.find(o => o.offerId.toNumber() !== 0);
        const offerId = offer.offerId;

        // Now add a time vector to the offer
        const currentTime = await testHelper.getCurrentClockTime();
        const startTime = new BN(currentTime + 3600); // 1 hour in future
        const startPrice = new BN(1000000); // 1 token
        const priceYield = new BN(5000);    // 50% yield (5000/10000)
        const priceFixDuration = new BN(3600); // 1 hour

        const tx = await testHelper.program.methods
            .addBuyOfferVector(
                offerId,
                startTime,
                startPrice,
                priceYield,
                priceFixDuration
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify the time vector was added
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);

        const updatedOffer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        expect(updatedOffer.offerId.toString()).toBe(offerId.toString());
        
        const vector = updatedOffer.vectors[0];
        expect(vector.vectorId.toString()).toBe("1");
        expect(vector.startTime.toString()).toBe(startTime.toString());
        expect(vector.validFrom.toString()).toBe(startTime.toString()); // valid_from should equal start_time when start_time is in future
        expect(vector.startPrice.toString()).toBe(startPrice.toString());
        expect(vector.priceYield.toString()).toBe(priceYield.toString());
        expect(vector.priceFixDuration.toString()).toBe(priceFixDuration.toString());
    });

    it("Should calculate valid_from as current time when start_time is in the past", async () => {
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
        const offerId = offer.offerId;

        const currentTime = await testHelper.getCurrentClockTime();
        const pastStartTime = new BN(currentTime - 3600); // 1 hour ago
        const startPrice = new BN(1000000);
        const priceYield = new BN(2500); // 25% yield
        const priceFixDuration = new BN(1800);

        await testHelper.program.methods
            .addBuyOfferVector(
                offerId,
                pastStartTime,
                startPrice,
                priceYield,
                priceFixDuration
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const updatedOffer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        const vector = updatedOffer.vectors[0];

        expect(vector.startTime.toString()).toBe(pastStartTime.toString());
        // valid_from should be approximately current time (within a few seconds)
        const validFromTime = parseInt(vector.validFrom.toString());
        expect(validFromTime).toBeGreaterThanOrEqual(currentTime);
        expect(validFromTime).toBeLessThanOrEqual(currentTime + 2); // Allow up to 2 seconds difference
    });

    it("Should auto-increment vector IDs correctly", async () => {
        const offerId = new BN(1);
        // Create buy offer
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add first vector
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

        // Add second vector (with later start_time)
        await testHelper.program.methods
            .addBuyOfferVector(
                offerId,
                new BN(currentTime + 3000),
                new BN(2000000),
                new BN(7500),
                new BN(1800)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Add third vector
        await testHelper.program.methods
            .addBuyOfferVector(
                offerId,
                new BN(currentTime + 5000),
                new BN(3000000),
                new BN(1000),
                new BN(900)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify vectors have correct auto-incremented IDs
        const [buyOffersPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("buy_offers")],
            ONREAPP_PROGRAM_ID
        );
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        
        expect(offer.vectors[0].vectorId.toString()).toBe("1");
        expect(offer.vectors[1].vectorId.toString()).toBe("2");
        expect(offer.vectors[2].vectorId.toString()).toBe("3");
    });

    it("Should reject zero offer_id", async () => {
        const currentTime = await testHelper.getCurrentClockTime();
        
        await expect(
            testHelper.program.methods
                .addBuyOfferVector(
                    new BN(0), // Invalid: zero offer_id
                    new BN(currentTime + 1000),
                    new BN(1000000),
                    new BN(5000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should reject zero start_time", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        await expect(
            testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(0), // Invalid: zero start_time
                    new BN(1000000),
                    new BN(5000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should reject zero start_price", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const currentTime = await testHelper.getCurrentClockTime();

        await expect(
            testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 1000),
                    new BN(0), // Invalid: zero start_price
                    new BN(5000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should reject zero price_yield", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const currentTime = await testHelper.getCurrentClockTime();

        await expect(
            testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 1000),
                    new BN(1000000),
                    new BN(0), // Invalid: zero price_yield
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should reject zero price_fix_duration", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const currentTime = await testHelper.getCurrentClockTime();

        await expect(
            testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 1000),
                    new BN(1000000),
                    new BN(5000),
                    new BN(0) // Invalid: zero price_fix_duration
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should reject start_time before latest existing vector start_time", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const currentTime = await testHelper.getCurrentClockTime();

        // Add first vector
        await testHelper.program.methods
            .addBuyOfferVector(
                offerId,
                new BN(currentTime + 2000),
                new BN(1000000),
                new BN(5000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Try to add vector with earlier start_time (should fail)
        await expect(
            testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 1000), // Invalid: before previous start_time
                    new BN(2000000),
                    new BN(7500),
                    new BN(1800)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid time range: start_time must be after the latest existing vector");
    });

    it("Should reject start_time equal to latest existing vector start_time", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const currentTime = await testHelper.getCurrentClockTime();
        const startTime = new BN(currentTime + 2000);

        // Add first vector
        await testHelper.program.methods
            .addBuyOfferVector(
                offerId,
                startTime,
                new BN(1000000),
                new BN(5000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Add vector with same start_time (should fail)
        await expect(testHelper.program.methods
            .addBuyOfferVector(
                offerId,
                startTime, // Same start_time - should be allowed
                new BN(2000000),
                new BN(7500),
                new BN(1800)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc()).rejects.toThrow("Invalid time range: start_time must be after the latest existing vector.");
    });

    it("Should reject adding vector to non-existent offer", async () => {
        const nonExistentOfferId = new BN(999999);
        const currentTime = await testHelper.getCurrentClockTime();
        
        await expect(
            testHelper.program.methods
                .addBuyOfferVector(
                    nonExistentOfferId,
                    new BN(currentTime + 1000),
                    new BN(1000000),
                    new BN(5000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Buy offer with the specified ID was not found");
    });

    it("Should reject when offer has maximum vectors", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const currentTime = await testHelper.getCurrentClockTime();
        const vectorTimeOffset = 1000;
        const startPrice = new BN(1000000);
        const priceYield = new BN(5000);
        const priceFixDuration = new BN(3600);

        // Add maximum number of vectors
        for (let i = 1; i <= MAX_SEGMENTS; i++) {
            const vectorStartTime = new BN(currentTime + (i * vectorTimeOffset));

            await testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    vectorStartTime,
                    startPrice,
                    priceYield,
                    priceFixDuration
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            console.log(`Added vector ${i}`);
        }

        // Try to add one more vector (should fail)
        const vectorStartTime = new BN(currentTime + ((MAX_SEGMENTS + 1) * vectorTimeOffset));

        await expect(
            testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    vectorStartTime,
                    startPrice,
                    priceYield,
                    priceFixDuration
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Cannot add more vectors: maximum limit reached");
    });

    it("Should handle large price and yield values correctly", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // Derive the buy offers account PDA
        const [buyOffersPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("buy_offers")],
            ONREAPP_PROGRAM_ID
        );

        const currentTime = await testHelper.getCurrentClockTime();

        // Use large values
        const largeStartPrice = new BN("999999999999999999"); // Large u64 value
        const largePriceYield = new BN("999999"); // 99.9999% yield (9999/10000)
        
        await testHelper.program.methods
            .addBuyOfferVector(
                offerId,
                new BN(currentTime + 1000),
                largeStartPrice,
                largePriceYield,
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify the vector was added with large values
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        const vector = offer.vectors[0];
        
        expect(vector.startPrice.toString()).toBe(largeStartPrice.toString());
        expect(vector.priceYield.toString()).toBe(largePriceYield.toString());
    });

    it("Should handle minimum valid values (1 for all fields)", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // Derive the buy offers account PDA
        const [buyOffersPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("buy_offers")],
            ONREAPP_PROGRAM_ID
        );

        await testHelper.program.methods
            .addBuyOfferVector(
                offerId,
                new BN(1), // Minimum valid start_time
                new BN(1), // Minimum valid start_price
                new BN(1), // Minimum valid price_yield
                new BN(1)  // Minimum valid price_fix_duration
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify the vector was added
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        const vector = offer.vectors[0];
        
        expect(vector.vectorId.toString()).toBe("1");
        expect(vector.startTime.toString()).toBe("1");
        // valid_from should be current time since start_time=1 is in the past
        const currentTime = await testHelper.getCurrentClockTime();
        const validFromTime = parseInt(vector.validFrom.toString());
        expect(validFromTime).toBeGreaterThanOrEqual(currentTime);
        expect(vector.startPrice.toString()).toBe("1");
        expect(vector.priceYield.toString()).toBe("1");
        expect(vector.priceFixDuration.toString()).toBe("1");
    });

    it("Should reject when called by non-boss", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const notBoss = testHelper.createUserAccount();
        const currentTime = await testHelper.getCurrentClockTime();
        
        await expect(
            testHelper.program.methods
                .addBuyOfferVector(
                    offerId,
                    new BN(currentTime + 1000),
                    new BN(1000000),
                    new BN(5000),
                    new BN(3600)
                )
                .accountsPartial({
                    state: testHelper.statePda,
                    boss: notBoss.publicKey,
                })
                .signers([notBoss])
                .rpc()
        ).rejects.toThrow(); // Should fail due to boss constraint
    });

    it("Should handle vectors added to multiple different offers", async () => {
        const offer1Id = new BN(1);
        const offer2Id = new BN(2);
        
        // Create two offers
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

        // Derive the buy offers account PDA
        const [buyOffersPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("buy_offers")],
            ONREAPP_PROGRAM_ID
        );

        const currentTime = await testHelper.getCurrentClockTime();

        // Add vectors to both offers
        await testHelper.program.methods
            .addBuyOfferVector(
                offer1Id,
                new BN(currentTime + 1000),
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
                offer2Id,
                new BN(currentTime + 1000),
                new BN(3000000),
                new BN(7500),
                new BN(1800)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        await testHelper.program.methods
            .addBuyOfferVector(
                offer1Id,
                new BN(currentTime + 3000),
                new BN(2000000),
                new BN(2500),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify each offer has its own vector ID sequence
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer1 = buyOfferAccount.offers.find(o => o.offerId.eq(offer1Id));
        const offer2 = buyOfferAccount.offers.find(o => o.offerId.eq(offer2Id));
        
        // Offer 1 should have vectors 1 and 2
        expect(offer1.vectors[0].vectorId.toString()).toBe("1");
        expect(offer1.vectors[1].vectorId.toString()).toBe("2");
        
        // Offer 2 should have vector 1 (independent sequence)
        expect(offer2.vectors[0].vectorId.toString()).toBe("1");
        
        // Verify prices are correct for each offer
        expect(offer1.vectors[0].startPrice.toString()).toBe("1000000");
        expect(offer2.vectors[0].startPrice.toString()).toBe("3000000");
        
        // Verify yields are correct
        expect(offer1.vectors[0].priceYield.toString()).toBe("5000"); // 50%
        expect(offer2.vectors[0].priceYield.toString()).toBe("7500"); // 75%
    });
});