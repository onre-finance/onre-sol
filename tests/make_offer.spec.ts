import { AddedProgram, startAnchor } from "solana-bankrun";
import { PublicKey } from "@solana/web3.js";
import { BN, Program } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { OnreApp } from "../target/types/onre_app";
import idl from "../target/idl/onre_app.json";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import { ONREAPP_PROGRAM_ID } from "./test_helper";
import { TestHelper } from "./test_helper";

describe("make offer", () => {
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

    test("Make an offer with one buy token", async () => {
        // given
        const { offerId, offerPda, offerSellTokenPda, offerBuyTokenPda, bossBuyTokenAccount } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(600e9)
        );
            
        const offerStartTime = Date.now() / 1000;
        const offerEndTime = offerStartTime + 7200;

        // when
        await testHelper.makeOfferOne({
            offerId, 
            buyTokenTotalAmount: 500e9, 
            sellTokenStartAmount: 200e9, 
            sellTokenEndAmount: 400e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration: 3600,
            sellTokenMint,
            buyTokenMint: buyToken1Mint,
        });

        // then
        const offerAccount = await testHelper.getOfferAccount(offerPda);
        expect(offerAccount.offerId.eq(offerId)).toBe(true);
        // sell token
        expect(offerAccount.sellTokenStartAmount.eq(new BN(200e9))).toBe(true);
        expect(offerAccount.sellTokenEndAmount.eq(new BN(400e9))).toBe(true);
        expect(offerAccount.sellTokenMint.toBase58()).toEqual(sellTokenMint.toBase58());
        // buy token
        expect(offerAccount.buyToken1.amount.eq(new BN(500e9))).toBe(true);
        expect(offerAccount.buyToken1.mint.toBase58()).toEqual(buyToken1Mint.toBase58());
        expect(offerAccount.buyToken2.amount.eq(new BN(0))).toBe(true);
        expect(offerAccount.buyToken2.mint.toBase58()).toEqual(SYSTEM_PROGRAM_ID.toBase58());
        // offer
        expect(offerAccount.priceFixDuration.eq(new BN(3600))).toBe(true);
        expect(offerAccount.offerStartTime.eq(new BN(offerStartTime))).toBe(true);
        expect(offerAccount.offerEndTime.eq(new BN(offerEndTime))).toBe(true);
        
        // 500 tokens substracted from boss buy token account
        await testHelper.expectTokenAccountAmountToBe(bossBuyTokenAccount, BigInt(100e9));

        // 500 tokens added to offer buy token account
        await testHelper.expectTokenAccountAmountToBe(offerBuyTokenPda, BigInt(500e9));

        // offerSellTokenPda stays empty
        await testHelper.expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(0));
    });

    test("Make offer with one buy token with price fix duration greater than offer duration should fail", async () => {
        // given
        const { offerId } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(600e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 7200;

        // when
        await expect(
            testHelper.makeOfferOne({
                offerId, 
                buyTokenTotalAmount: 500e9, 
                sellTokenStartAmount: 200e9, 
                sellTokenEndAmount: 400e9, 
                offerStartTime, 
                offerEndTime, 
                priceFixDuration: 7201,
                sellTokenMint,
                buyTokenMint: buyToken1Mint
            })
        ).rejects.toThrow(RegExp(".*InvalidPriceFixDuration.*"));
    });

    test("Make offer with one buy token with zero price fix duration should fail", async () => {
        // given
        const { offerId } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(600e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 7200;

        // when
        await expect(
            testHelper.makeOfferOne({
                offerId, 
                buyTokenTotalAmount: 500e9, 
                sellTokenStartAmount: 200e9, 
                sellTokenEndAmount: 400e9, 
                offerStartTime, 
                offerEndTime, 
                priceFixDuration: 0,
                sellTokenMint,
                buyTokenMint: buyToken1Mint
            })
        ).rejects.toThrow(RegExp(".*InvalidPriceFixDuration.*"));
    });

    test("Make offer with two buy tokens with price fix duration greater than offer duration should fail", async () => {
        // given
        const { offerId } = testHelper.createTwoTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            buyToken2Mint, BigInt(0), 
            boss, BigInt(600e9), BigInt(600e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 7200;

        // when
        await expect(
            testHelper.makeOfferTwo({
                offerId, 
                buyToken1TotalAmount: 500e9, 
                buyToken2TotalAmount: 300e9, 
                sellTokenStartAmount: 200e9, 
                sellTokenEndAmount: 400e9, 
                offerStartTime, 
                offerEndTime, 
                priceFixDuration: 7201,
                sellTokenMint,
                buyToken1Mint,
                buyToken2Mint,
            })
        ).rejects.toThrow(RegExp(".*InvalidPriceFixDuration.*"));
    });

    test("Make offer with two buy token with zero price fix duration should fail", async () => {
        // given
        const { offerId } = testHelper.createTwoTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            buyToken2Mint, BigInt(0), 
            boss, BigInt(600e9), BigInt(600e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 7200;

        // when
        await expect(
            testHelper.makeOfferTwo({
                offerId, 
                buyToken1TotalAmount: 500e9, 
                buyToken2TotalAmount: 300e9, 
                sellTokenStartAmount: 200e9, 
                sellTokenEndAmount: 400e9, 
                offerStartTime, 
                offerEndTime, 
                priceFixDuration: 0,
                sellTokenMint,
                buyToken1Mint,
                buyToken2Mint,
            })
        ).rejects.toThrow(RegExp(".*InvalidPriceFixDuration.*"));
    });

    test("Make an offer with same sell_token_start_amount and sell_token_end_amount should succeed", async () => {        
        // given
        const { offerId, offerPda } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(1000e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 7200;

        // when
        await testHelper.makeOfferOne({
            offerId, 
            buyTokenTotalAmount: 500e9, 
            sellTokenStartAmount: 200e9, 
            sellTokenEndAmount: 200e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration: 3600,
            sellTokenMint,
            buyTokenMint: buyToken1Mint
        });

        // then
        const offerAccount = await testHelper.getOfferAccount(offerPda);
        expect(offerAccount.sellTokenStartAmount.eq(new BN(200e9))).toBe(true);
        expect(offerAccount.sellTokenEndAmount.eq(new BN(200e9))).toBe(true);
    });

    test("Make an offer with two buy tokens should succeed", async () => {
        // given
        const { offerId, offerPda, bossBuyTokenAccount1, bossBuyTokenAccount2, offerBuyToken1Pda, offerBuyToken2Pda, offerSellTokenPda } = testHelper.createTwoTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            buyToken2Mint, BigInt(0), 
            boss, BigInt(600e9), BigInt(600e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 7200;

        // when
        await testHelper.makeOfferTwo({
            offerId, 
            buyToken1TotalAmount: 500e9, 
            buyToken2TotalAmount: 300e9, 
            sellTokenStartAmount: 200e9, 
            sellTokenEndAmount: 400e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration: 3600,
            sellTokenMint,
            buyToken1Mint,
            buyToken2Mint,
        });

        // then
        const offerAccount = await testHelper.getOfferAccount(offerPda);
        expect(offerAccount.offerId.eq(offerId)).toBe(true);
        // sell token
        expect(offerAccount.sellTokenStartAmount.eq(new BN(200e9))).toBe(true);
        expect(offerAccount.sellTokenEndAmount.eq(new BN(400e9))).toBe(true);
        expect(offerAccount.sellTokenMint.toBase58()).toEqual(sellTokenMint.toBase58());
        // buy token
        expect(offerAccount.buyToken1.amount.eq(new BN(500e9))).toBe(true);
        expect(offerAccount.buyToken1.mint.toBase58()).toEqual(buyToken1Mint.toBase58());
        expect(offerAccount.buyToken2.amount.eq(new BN(300e9))).toBe(true);
        expect(offerAccount.buyToken2.mint.toBase58()).toEqual(buyToken2Mint.toBase58());
        // offer
        expect(offerAccount.priceFixDuration.eq(new BN(3600))).toBe(true);
        expect(offerAccount.offerStartTime.eq(new BN(offerStartTime))).toBe(true);
        expect(offerAccount.offerEndTime.eq(new BN(offerEndTime))).toBe(true);
        
        // 500 tokens substracted from boss buy token account 1
        await testHelper.expectTokenAccountAmountToBe(bossBuyTokenAccount1, BigInt(100e9));

        // 300 tokens substracted from boss buy token account 2
        await testHelper.expectTokenAccountAmountToBe(bossBuyTokenAccount2, BigInt(300e9));

        // 500 tokens added to offer buy token account 1
        await testHelper.expectTokenAccountAmountToBe(offerBuyToken1Pda, BigInt(500e9));

        // 300 tokens added to offer buy token account 2
        await testHelper.expectTokenAccountAmountToBe(offerBuyToken2Pda, BigInt(300e9));

        // offerSellTokenPda stays empty
        await testHelper.expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(0));
    });

    test("Make offer with one buy token with end time before start time should fail", async () => {
        // given
        const { offerId } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(600e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime - 60;

        // when
        await expect(
            testHelper.makeOfferOne({
                offerId, 
                buyTokenTotalAmount: 500e9, 
                sellTokenStartAmount: 200e9, 
                sellTokenEndAmount: 400e9, 
                offerStartTime, 
                offerEndTime, 
                priceFixDuration: 60,
                sellTokenMint,
                buyTokenMint: buyToken1Mint,
            })
        ).rejects.toThrow(RegExp(".*InvalidOfferTime.*"));
    });

    test("Make offer with two buy tokens with end time before start time should fail", async () => {
        // given
        const { offerId } = testHelper.createTwoTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            buyToken2Mint, BigInt(0), 
            boss, BigInt(600e9), BigInt(600e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime - 1000;

        // when
        await expect(
            testHelper.makeOfferTwo({
                offerId, 
                buyToken1TotalAmount: 500e9, 
                buyToken2TotalAmount: 300e9, 
                sellTokenStartAmount: 200e9, 
                sellTokenEndAmount: 400e9, 
                offerStartTime, 
                offerEndTime, 
                priceFixDuration: 60,
                sellTokenMint,
                buyToken1Mint,
                buyToken2Mint,
            })
        ).rejects.toThrow(RegExp(".*InvalidOfferTime.*"));
    });

    test("Make offer with existing offer_id should fail", async () => {
        // given
        const { offerId } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(600e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 7200;

        // when
        await testHelper.makeOfferOne({
                offerId, 
                buyTokenTotalAmount: 500e9, 
                sellTokenStartAmount: 200e9, 
                sellTokenEndAmount: 400e9, 
                offerStartTime, 
                offerEndTime, 
                priceFixDuration: 3600,
                sellTokenMint,
                buyTokenMint: buyToken1Mint,
            });

        // then
        await expect(
            testHelper.makeOfferOne({
                offerId, 
                buyTokenTotalAmount: 500e9, 
                sellTokenStartAmount: 200e9, 
                sellTokenEndAmount: 400e9, 
                offerStartTime, 
                offerEndTime, 
                priceFixDuration: 3600,
                sellTokenMint,
                buyTokenMint: buyToken1Mint,
            })
        ).rejects.toThrow();    
    });

    test("Make offer with offer time not being a multiple of price_fix_duration should fail", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const userSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = testHelper.createTokenAccount(buyToken1Mint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda, offerSellTokenPda, offerBuyTokenPda } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(10e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 216_000; // 2.5 days

        // make offer
        await expect(testHelper.makeOfferOne({
            offerId, 
            buyTokenTotalAmount: 10e9, 
            sellTokenStartAmount: 10e9, 
            sellTokenEndAmount: 20e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration: 86400, // 1 day
            sellTokenMint,
            buyTokenMint: buyToken1Mint,
        })).rejects.toThrow(RegExp(".*InvalidOfferTime.*"));
    })
});