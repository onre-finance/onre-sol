import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "./test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../target/idl/onreapp.json";

describe("Add Buy Offer Time Segment", () => {
    let testHelper: TestHelper;
    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;
    let boss: PublicKey;

    beforeAll(async () => {
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
        
        // Initialize program
        await program.methods.initialize().accounts({ boss }).rpc();
    });

    it("Should create a buy offer and add a time segment", async () => {
        const offerId = new BN(1);
        
        // First create a buy offer using testHelper
        await testHelper.makeBuyOffer({
            offerId: offerId,
            tokenInMint,
            tokenOutMint,
        });

        // Derive the buy offers account PDA
        const [buyOffersPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("buy_offers")],
            ONREAPP_PROGRAM_ID
        );

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

        const offer = buyOfferAccount.offers[0];
        expect(offer.offerId.toString()).toBe(offerId.toString());
        
        const segment = offer.timeSegments[0];
        expect(segment.segmentId.toString()).toBe("1");
        expect(segment.startTime.toString()).toBe(startTime.toString());
        expect(segment.endTime.toString()).toBe(endTime.toString());
        expect(segment.startPrice.toString()).toBe(startPrice.toString());
        expect(segment.endPrice.toString()).toBe(endPrice.toString());
        expect(segment.priceFixDuration.toString()).toBe(priceFixDuration.toString());
    });
});