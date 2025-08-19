import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { OnreApp } from "../../target/types/onre_app";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onre_app.json";

describe("make offer", () => {
    let testHelper: TestHelper;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    let boss: PublicKey;

    beforeAll(async () => {
        const programInfo: AddedProgram = {
            programId: ONREAPP_PROGRAM_ID,
            name: "onreapp",
        };

        const context = await startAnchor("", [programInfo], []);

        const provider = new BankrunProvider(context);
        const program = new Program<OnreApp>(
            idl,
            provider,
        );

        testHelper = new TestHelper(context, program);

        boss = provider.wallet.publicKey;
        
        // Create mints
        tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        tokenOutMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        
        await program.methods.initialize().accounts({ boss }).rpc();
    });

    test("Make a buy offer should succeed", async () => {
        // given
        const { offerAuthority, buyOfferAccountPda, offerTokenInPda, offerTokenOutPda, bossTokenInAccount } = testHelper.createBuyOfferAccounts(
            tokenInMint, BigInt(0), 
            tokenOutMint, BigInt(0), 
            boss, BigInt(100_000e9),
        );

        // when
        await testHelper.makeBuyOffer({
            offerId: new BN(1),
            tokenInAmount: 10e9,
            segmentId: 1,
            startPrice: 1e9,
            endPrice: 2e9,
            startTime: 10_000,
            endTime: 20_000,
            priceFixDuration: 1000, // 10 intervals
            tokenInMint,
            tokenOutMint,
        });

        // // then
        // const buyOfferAccountData = await testHelper.program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        
        // expect(buyOfferAccountData.count.toNumber()).toBe(1);
        
        // const firstOffer = buyOfferAccountData.offers[0];
        // expect(firstOffer.offerId.toNumber()).toBe(1);
        // expect(firstOffer.tokenInMint.toString()).toBe(tokenInMint.toString());
        // expect(firstOffer.tokenOutMint.toString()).toBe(tokenOutMint.toString());
        
        // const timeSegment = firstOffer.timeSegments[0];
        // expect(timeSegment.segmentId.toNumber()).toBe(1);
        // expect(timeSegment.startPrice.toNumber()).toBe(1e9); // 1 billion
        // expect(timeSegment.endPrice.toNumber()).toBe(2e9);   // 2 billion
        // expect(timeSegment.startTime.toNumber()).toBe(10_000);
        // expect(timeSegment.endTime.toNumber()).toBe(20_000);
        // expect(timeSegment.priceFixDuration.toNumber()).toBe(1000);
    });
});