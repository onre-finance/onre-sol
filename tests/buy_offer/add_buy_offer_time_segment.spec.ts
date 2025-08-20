import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

const MAX_SEGMENTS = 10;

describe("Add Buy Offer Time Segment", () => {
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

    it("Should create a buy offer and add a time segment", async () => {
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

        // Now add a time segment to the offer
        const startTime = new BN(Math.floor(Date.now() / 1000));
        const endTime = new BN(Math.floor(Date.now() / 1000) + 86400); // 24 hours later
        const startPrice = new BN(1000000); // 1 token
        const endPrice = new BN(2000000);   // 2 tokens
        const priceFixDuration = new BN(3600); // 1 hour

        const tx = await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                startTime,
                endTime,
                startPrice,
                endPrice,
                priceFixDuration
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        console.log("Add time segment transaction signature:", tx);

        // Verify the time segment was added
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);

        const updatedOffer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        expect(updatedOffer.offerId.toString()).toBe(offerId.toString());
        
        const segment = updatedOffer.timeSegments[0];
        expect(segment.segmentId.toString()).toBe("1");
        expect(segment.startTime.toString()).toBe(startTime.toString());
        expect(segment.endTime.toString()).toBe(endTime.toString());
        expect(segment.startPrice.toString()).toBe(startPrice.toString());
        expect(segment.endPrice.toString()).toBe(endPrice.toString());
        expect(segment.priceFixDuration.toString()).toBe(priceFixDuration.toString());
    });

    it("Should auto-increment segment IDs correctly", async () => {
        const offerId = new BN(1);
        // Create buy offer
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // Derive the buy offers account PDA
        const [buyOffersPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("buy_offers")],
            ONREAPP_PROGRAM_ID
        );

        // Add first segment
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                new BN(1000),
                new BN(2000),
                new BN(1000000),
                new BN(2000000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Add second segment (non-overlapping)
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                new BN(3000),
                new BN(4000),
                new BN(2000000),
                new BN(3000000),
                new BN(1800)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Add third segment
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                new BN(5000),
                new BN(6000),
                new BN(3000000),
                new BN(4000000),
                new BN(900)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify segments have correct auto-incremented IDs
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        
        expect(offer.timeSegments[0].segmentId.toString()).toBe("1");
        expect(offer.timeSegments[1].segmentId.toString()).toBe("2");
        expect(offer.timeSegments[2].segmentId.toString()).toBe("3");
    });

    it("Should reject zero offer_id", async () => {
        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    new BN(0), // Invalid: zero offer_id
                    new BN(1000),
                    new BN(2000),
                    new BN(1000000),
                    new BN(2000000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should reject zero start_time", async () => {
        const offerId = new BN(10);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(0), // Invalid: zero start_time
                    new BN(1000),
                    new BN(1000000),
                    new BN(2000000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should reject zero end_time", async () => {
        const offerId = new BN(11);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(1000),
                    new BN(0), // Invalid: zero end_time
                    new BN(1000000),
                    new BN(2000000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should reject zero start_price", async () => {
        const offerId = new BN(12);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(1000),
                    new BN(2000),
                    new BN(0), // Invalid: zero start_price
                    new BN(2000000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should reject zero end_price", async () => {
        const offerId = new BN(13);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(1000),
                    new BN(2000),
                    new BN(1000000),
                    new BN(0), // Invalid: zero end_price
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should reject zero price_fix_duration", async () => {
        const offerId = new BN(14);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(1000),
                    new BN(2000),
                    new BN(1000000),
                    new BN(2000000),
                    new BN(0) // Invalid: zero price_fix_duration
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid input: values cannot be zero");
    });

    it("Should reject start_time >= end_time", async () => {
        const offerId = new BN(15);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // Test equal times
        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(2000),
                    new BN(2000), // Invalid: equal to start_time
                    new BN(1000000),
                    new BN(2000000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid time range");

        // Test end_time < start_time
        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(3000),
                    new BN(2000), // Invalid: less than start_time
                    new BN(1000000),
                    new BN(2000000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid time range");
    });

    it("Should reject start_price >= end_price", async () => {
        const offerId = new BN(16);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // Test equal prices
        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(1000),
                    new BN(2000),
                    new BN(2000000),
                    new BN(2000000), // Invalid: equal to start_price
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid price range");

        // Test end_price < start_price
        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(1000),
                    new BN(2000),
                    new BN(3000000),
                    new BN(2000000), // Invalid: less than start_price
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Invalid price range");
    });

    it("Should reject adding segment to non-existent offer", async () => {
        const nonExistentOfferId = new BN(999999);
        
        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    nonExistentOfferId,
                    new BN(1000),
                    new BN(2000),
                    new BN(1000000),
                    new BN(2000000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Buy offer with the specified ID was not found");
    });

    it("Should reject completely overlapping segments", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // Add first segment
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                new BN(1000),
                new BN(3000),
                new BN(1000000),
                new BN(2000000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Try to add completely overlapping segment
        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(1500),
                    new BN(2500), // Completely inside first segment
                    new BN(2000000),
                    new BN(3000000),
                    new BN(1800)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Time segment overlaps with existing segments");
    });

    it("Should reject partial overlap at start", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // Add first segment
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                new BN(1000),
                new BN(3000),
                new BN(1000000),
                new BN(2000000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Try to add segment that overlaps at the beginning
        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(500),
                    new BN(1500), // Overlaps: starts before, ends inside
                    new BN(2000000),
                    new BN(3000000),
                    new BN(1800)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Time segment overlaps with existing segments");
    });

    it("Should reject partial overlap at end", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // Add first segment
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                new BN(1000),
                new BN(3000),
                new BN(1000000),
                new BN(2000000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Try to add segment that overlaps at the end
        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(2500),
                    new BN(4000), // Overlaps: starts inside, ends after
                    new BN(2000000),
                    new BN(3000000),
                    new BN(1800)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Time segment overlaps with existing segments");
    });

    it("Should reject segment that encompasses existing segment", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        // Add first segment
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                new BN(2000),
                new BN(3000),
                new BN(1000000),
                new BN(2000000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Try to add segment that encompasses the existing one
        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(1000),
                    new BN(4000), // Encompasses existing segment
                    new BN(2000000),
                    new BN(3000000),
                    new BN(1800)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Time segment overlaps with existing segments");
    });

    it("Should allow adjacent segments (boundary testing)", async () => {
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

        // Add first segment
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                new BN(1000),
                new BN(2000),
                new BN(1000000),
                new BN(2000000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Add adjacent segment (should succeed)
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                new BN(2000), // Starts exactly where previous ends
                new BN(3000),
                new BN(2000000),
                new BN(3000000),
                new BN(1800)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify both segments exist
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        
        expect(offer.timeSegments[0].segmentId.toString()).toBe("1");
        expect(offer.timeSegments[1].segmentId.toString()).toBe("2");
    });

    it("Should reject when offer has maximum segments", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const startTime = 1000;
        const segmentDuration = 1000;
        const startPrice = new BN(1000000);
        const endPrice = new BN(2000000);
        const priceFixDuration = new BN(3600);

        // Add maximum number of segments
        for (let i = 1; i <= MAX_SEGMENTS; i++) {
            const offset = i * segmentDuration;
            const segmentStartTime = new BN(startTime + offset);
            const segmentEndTime = segmentStartTime.addn(segmentDuration);

            await testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    segmentStartTime,
                    segmentEndTime,
                    startPrice,
                    endPrice,
                    priceFixDuration
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();

            console.log(`Added segment ${i},`);
        }

        // Try to add one more segment (should fail)
        const offset = (MAX_SEGMENTS + 1) * segmentDuration;
        const segmentStartTime = new BN(startTime + offset);
        const segmentEndTime = segmentStartTime.addn(segmentDuration);

        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    segmentStartTime,
                    segmentEndTime,
                    startPrice,
                    endPrice,
                    priceFixDuration
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc()
        ).rejects.toThrow("Cannot add more segments: maximum limit reached");
    });

    it("Should handle large timestamp values correctly", async () => {
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

        // Use large timestamp values (near u64 limit but reasonable for timestamps)
        const largeStartTime = new BN("1893456000"); // Year 2030
        const largeEndTime = new BN("1924992000");   // Year 2031
        
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                largeStartTime,
                largeEndTime,
                new BN(1000000),
                new BN(2000000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify the segment was added with large values
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        const segment = offer.timeSegments[0];
        
        expect(segment.startTime.toString()).toBe(largeStartTime.toString());
        expect(segment.endTime.toString()).toBe(largeEndTime.toString());
    });

    it("Should handle large price values correctly", async () => {
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

        // Use large price values
        const largeStartPrice = new BN("999999999999999999"); // Large u64 value
        const largeEndPrice = new BN("1000000000000000000");  // Even larger
        
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offerId,
                new BN(1000),
                new BN(2000),
                largeStartPrice,
                largeEndPrice,
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify the segment was added with large values
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        const segment = offer.timeSegments[0];
        
        expect(segment.startPrice.toString()).toBe(largeStartPrice.toString());
        expect(segment.endPrice.toString()).toBe(largeEndPrice.toString());
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
            .addBuyOfferTimeSegment(
                offerId,
                new BN(1), // Minimum valid start_time
                new BN(2), // Minimum valid end_time
                new BN(1), // Minimum valid start_price
                new BN(2), // Minimum valid end_price
                new BN(1)  // Minimum valid price_fix_duration
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify the segment was added
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        const segment = offer.timeSegments[0];
        
        expect(segment.segmentId.toString()).toBe("1");
        expect(segment.startTime.toString()).toBe("1");
        expect(segment.endTime.toString()).toBe("2");
        expect(segment.startPrice.toString()).toBe("1");
        expect(segment.endPrice.toString()).toBe("2");
        expect(segment.priceFixDuration.toString()).toBe("1");
    });

    it("Should reject when called by non-boss", async () => {
        const offerId = new BN(1);
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint,
        });

        const notBoss = testHelper.createUserAccount();
        
        await expect(
            testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(1000),
                    new BN(2000),
                    new BN(1000000),
                    new BN(2000000),
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

    it("Should handle multiple segments with complex time patterns", async () => {
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

        // Add segments in non-sequential time order
        const segments = [
            { start: 5000, end: 6000 },  // Third chronologically
            { start: 1000, end: 2000 },  // First chronologically  
            { start: 3000, end: 4000 },  // Second chronologically
            { start: 7000, end: 8000 },  // Fourth chronologically
        ];

        for (let i = 0; i < segments.length; i++) {
            await testHelper.program.methods
                .addBuyOfferTimeSegment(
                    offerId,
                    new BN(segments[i].start),
                    new BN(segments[i].end),
                    new BN(1000000 + i * 100000),
                    new BN(2000000 + i * 100000),
                    new BN(3600)
                )
                .accounts({
                    state: testHelper.statePda,
                })
                .rpc();
        }

        // Verify all segments were added with correct sequential IDs
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer = buyOfferAccount.offers.find(o => o.offerId.eq(offerId));
        
        for (let i = 0; i < segments.length; i++) {
            expect(offer.timeSegments[i].segmentId.toString()).toBe((i + 1).toString());
            expect(offer.timeSegments[i].startTime.toString()).toBe(segments[i].start.toString());
            expect(offer.timeSegments[i].endTime.toString()).toBe(segments[i].end.toString());
        }
    });

    it("Should handle segments added to multiple different offers", async () => {
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

        // Add segments to both offers
        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offer1Id,
                new BN(1000),
                new BN(2000),
                new BN(1000000),
                new BN(2000000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offer2Id,
                new BN(1000),
                new BN(2000),
                new BN(3000000),
                new BN(4000000),
                new BN(1800)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        await testHelper.program.methods
            .addBuyOfferTimeSegment(
                offer1Id,
                new BN(3000),
                new BN(4000),
                new BN(2000000),
                new BN(3000000),
                new BN(3600)
            )
            .accounts({
                state: testHelper.statePda,
            })
            .rpc();

        // Verify each offer has its own segment ID sequence
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOffersPda);
        const offer1 = buyOfferAccount.offers.find(o => o.offerId.eq(offer1Id));
        const offer2 = buyOfferAccount.offers.find(o => o.offerId.eq(offer2Id));
        
        // Offer 1 should have segments 1 and 2
        expect(offer1.timeSegments[0].segmentId.toString()).toBe("1");
        expect(offer1.timeSegments[1].segmentId.toString()).toBe("2");
        
        // Offer 2 should have segment 1 (independent sequence)
        expect(offer2.timeSegments[0].segmentId.toString()).toBe("1");
        
        // Verify prices are correct for each offer
        expect(offer1.timeSegments[0].startPrice.toString()).toBe("1000000");
        expect(offer2.timeSegments[0].startPrice.toString()).toBe("3000000");
    });
});