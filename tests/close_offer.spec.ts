import { BankrunProvider } from "anchor-bankrun";
import { startAnchor } from "anchor-bankrun";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "./test_helper";
import { AddedProgram } from "solana-bankrun";
import { Program } from "@coral-xyz/anchor";
import { OnreApp } from "../target/types/onre_app";
import idl from "../target/idl/onre_app.json";


describe("close offer", () => {
    let testHelper: TestHelper;

    let sellTokenMint: PublicKey;
    let buyToken1Mint: PublicKey;
    let buyToken2Mint: PublicKey;

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
        sellTokenMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        buyToken1Mint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        buyToken2Mint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        
        await program.methods.initialize().accounts({ boss }).rpc();
    });
    
    test("closing an offer with one buy token should refund all buy tokens", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const userSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = testHelper.createTokenAccount(buyToken1Mint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda, offerSellTokenPda, offerBuyTokenPda, bossBuyTokenAccount } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(10e9)
        );

        const priceFixDuration = 86400; // 1 day
        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + (priceFixDuration * 3); // 3 days

        // make offer
        await testHelper.makeOfferOne({
            offerId, 
            buyTokenTotalAmount: 10e9, 
            sellTokenStartAmount: 10e9, 
            sellTokenEndAmount: 20e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration,
            sellTokenMint,
            buyTokenMint: buyToken1Mint,
        })

        // time travel to last interval
        await testHelper.advanceClockBy(priceFixDuration * 2.5); // + 2.5 days

        const bossSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, boss, BigInt(0), true);

        // when
        await testHelper.closeOfferOne(offerPda);

        // then
        await testHelper.expectTokenAccountAmountToBe(bossBuyTokenAccount, BigInt(10e9));
        await testHelper.expectTokenAccountAmountToBe(bossSellTokenAccount, BigInt(0e9));
    });

    test("closing an offer with two buy tokens should refund all buy tokens", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const userSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = testHelper.createTokenAccount(buyToken1Mint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda, offerSellTokenPda, offerBuyToken1Pda, offerBuyToken2Pda, bossBuyTokenAccount1, bossBuyTokenAccount2 } = testHelper.createTwoTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            buyToken2Mint, BigInt(0),
            boss, BigInt(10e9), BigInt(10e9)
        );

        const priceFixDuration = 86400; // 1 day
        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + (priceFixDuration * 3); // 3 days

        // make offer
        await testHelper.makeOfferTwo({
            offerId, 
            buyToken1TotalAmount: 10e9, 
            buyToken2TotalAmount: 10e9, 
            sellTokenStartAmount: 10e9, 
            sellTokenEndAmount: 20e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration,
            sellTokenMint,
            buyToken1Mint,
            buyToken2Mint,
        })

        // time travel to last interval
        await testHelper.advanceClockBy(priceFixDuration * 2.5); // + 2.5 days

        const bossSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, boss, BigInt(0), true);

        // when
        await testHelper.closeOfferTwo(offerPda);

        // then
        await testHelper.expectTokenAccountAmountToBe(bossBuyTokenAccount1, BigInt(10e9));
        await testHelper.expectTokenAccountAmountToBe(bossBuyTokenAccount2, BigInt(10e9));
        await testHelper.expectTokenAccountAmountToBe(bossSellTokenAccount, BigInt(0e9));
    });

    test("close_offer_two called on an offer with one buy token should fail", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const userSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyTokenAccount = testHelper.createTokenAccount(buyToken1Mint, user.publicKey, BigInt(0), true);
        
        // create offer accounts
        const { offerId, offerPda, offerAuthority } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(10e9)
        );
        testHelper.createTokenAccount(SystemProgram.programId, offerAuthority, BigInt(0), true);
        testHelper.createTokenAccount(SystemProgram.programId, boss, BigInt(10e9));

        const priceFixDuration = 86400; // 1 day
        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + (priceFixDuration * 3); // 3 days

        // make offer
        await testHelper.makeOfferOne({
            offerId, 
            buyTokenTotalAmount: 10e9, 
            sellTokenStartAmount: 10e9, 
            sellTokenEndAmount: 20e9,
            offerStartTime, 
            offerEndTime, 
            priceFixDuration,
            sellTokenMint,
            buyTokenMint: buyToken1Mint,
        })
        
        // when
        await expect(testHelper.closeOfferTwo(offerPda)).rejects.toThrow(RegExp(".*InvalidCloseOffer.*"));
    });

    test("close_offer_one called on an offer with two buy tokens should fail", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const userSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyTokenAccount = testHelper.createTokenAccount(buyToken1Mint, user.publicKey, BigInt(0), true);
        
        // create offer accounts
        const { offerId, offerPda, offerAuthority } = testHelper.createTwoTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            buyToken2Mint, BigInt(0),
            boss, BigInt(10e9), BigInt(10e9)
        );

        const priceFixDuration = 86400; // 1 day
        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + (priceFixDuration * 3); // 3 days

        // make offer
        await testHelper.makeOfferTwo({
            offerId, 
            buyToken1TotalAmount: 10e9, 
            buyToken2TotalAmount: 10e9, 
            sellTokenStartAmount: 10e9, 
            sellTokenEndAmount: 20e9,
            offerStartTime, 
            offerEndTime, 
            priceFixDuration,
            sellTokenMint,
            buyToken1Mint,
            buyToken2Mint,
        })
        
        // when
        await expect(testHelper.closeOfferOne(offerPda)).rejects.toThrow(RegExp(".*InvalidCloseOffer.*"));
    })
})