import { AddedProgram, startAnchor } from "solana-bankrun";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { OnreApp } from "../target/types/onre_app";
import idl from "../target/idl/onre_app.json";
import { ONREAPP_PROGRAM_ID, TestHelper } from "./test_helper";

describe("take offer", () => {
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

    test("Take offer with one buy token in first interval should succeed", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const userSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = testHelper.createTokenAccount(buyToken1Mint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda, offerSellTokenPda, offerBuyTokenPda } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(600e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 7200;

        // make offer
        await testHelper.makeOfferOne({
            offerId, 
            buyTokenTotalAmount: 100e9, 
            sellTokenStartAmount: 100e9, 
            sellTokenEndAmount: 200e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration: 3600,
            sellTokenMint,
            buyTokenMint: buyToken1Mint,
        })

        // when
        await testHelper.takeOfferOne({
            sellTokenAmount: 10e9,
            offerPda,
            user,
        })

        // then
        await testHelper.expectTokenAccountAmountToBe(userSellTokenAccount, BigInt(90e9));
        await testHelper.expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(10e9));
        await testHelper.expectTokenAccountAmountToBe(userBuyToken1Account, BigInt(6666666666));
        await testHelper.expectTokenAccountAmountToBe(offerBuyTokenPda, BigInt(93333333334));
    });

    test("Take offer with one buy token in second interval should succeed", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const userSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = testHelper.createTokenAccount(buyToken1Mint, user.publicKey, BigInt(0), true);
        
        // create offer accounts
        const { offerId, offerPda, offerSellTokenPda, offerBuyTokenPda } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(600e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 7200;

        // make offer
        await testHelper.makeOfferOne({
            offerId, 
            buyTokenTotalAmount: 100e9, 
            sellTokenStartAmount: 100e9, 
            sellTokenEndAmount: 200e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration: 3600,
            sellTokenMint,
            buyTokenMint: buyToken1Mint,
        })

        // time travel to next interval
        await testHelper.advanceClockBy(3600);

        // when
        await testHelper.takeOfferOne({
            sellTokenAmount: 10e9,
            offerPda,
            user,
        })
            
        // then
        await testHelper.expectTokenAccountAmountToBe(userSellTokenAccount, BigInt(90e9));
        await testHelper.expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(10e9));
        await testHelper.expectTokenAccountAmountToBe(userBuyToken1Account, BigInt(5e9));
        await testHelper.expectTokenAccountAmountToBe(offerBuyTokenPda, BigInt(95e9));  
    });

    test("Take offer with zero amount should fail", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;

        const userOnreTokenAccount = testHelper.createTokenAccount(onreTokenMint, user.publicKey, BigInt(100e9), true);
        const userUsdcTokenAccount = testHelper.createTokenAccount(usdcTokenMint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda,offerBuyTokenPda, offerSellTokenPda } = testHelper.createOneTokenOfferAccounts(
            onreTokenMint, BigInt(0), 
            usdcTokenMint, BigInt(0), 
            boss, BigInt(200e9)
        );
        const offerUsdcTokenPda = offerBuyTokenPda;
        const offerOnreTokenPda = offerSellTokenPda;

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 7200; // 2 hours
        const priceFixDuration = 3600; // 1 hour

        // make offer
        await testHelper.makeOfferOne({
            offerId,
            buyTokenTotalAmount: 100e9,
            sellTokenStartAmount: 100e9,
            sellTokenEndAmount: 200e9,
            offerStartTime,
            offerEndTime,
            priceFixDuration,
            sellTokenMint: onreTokenMint,
            buyTokenMint: usdcTokenMint,
        })

        // when
        await expect(testHelper.takeOfferOne({
            sellTokenAmount: 0,
            offerPda,
            user,
        })).rejects.toThrow(RegExp(".*ZeroBuyTokenAmount.*"));

        // then
        await testHelper.expectTokenAccountAmountToBe(userOnreTokenAccount, BigInt(100e9));
        await testHelper.expectTokenAccountAmountToBe(userUsdcTokenAccount, BigInt(0));
        await testHelper.expectTokenAccountAmountToBe(offerUsdcTokenPda, BigInt(100e9));
        await testHelper.expectTokenAccountAmountToBe(offerOnreTokenPda, BigInt(0));
        
    });

    test("Take offer with maximum amount in first interval should succeed", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;

        const userOnreTokenAccount = testHelper.createTokenAccount(onreTokenMint, user.publicKey, BigInt(100e9), true);
        const userUsdcTokenAccount = testHelper.createTokenAccount(usdcTokenMint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda,offerBuyTokenPda, offerSellTokenPda } = testHelper.createOneTokenOfferAccounts(
            onreTokenMint, BigInt(0), 
            usdcTokenMint, BigInt(0), 
            boss, BigInt(200e9)
        );
        const offerUsdcTokenPda = offerBuyTokenPda;
        const offerOnreTokenPda = offerSellTokenPda;

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 10800; // 3 hours
        const priceFixDuration = 3600; // 1 hour

        // make offer
        await testHelper.makeOfferOne({
            offerId,
            buyTokenTotalAmount: 100e9,
            sellTokenStartAmount: 50e9,
            sellTokenEndAmount: 200e9,
            offerStartTime,
            offerEndTime,
            priceFixDuration,
            sellTokenMint: onreTokenMint,
            buyTokenMint: usdcTokenMint,
        })

        // when
        await testHelper.takeOfferOne({
            sellTokenAmount: 100e9,
            offerPda,
            user,
        })

        // then
        await testHelper.expectTokenAccountAmountToBe(userOnreTokenAccount, BigInt(0));
        await testHelper.expectTokenAccountAmountToBe(userUsdcTokenAccount, BigInt(100e9));
        await testHelper.expectTokenAccountAmountToBe(offerUsdcTokenPda, BigInt(0));
        await testHelper.expectTokenAccountAmountToBe(offerOnreTokenPda, BigInt(100e9));
    });

    test("Take offer with maximum amount in last interval should succeed", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;

        const userOnreTokenAccount = testHelper.createTokenAccount(onreTokenMint, user.publicKey, BigInt(100e9), true);
        const userUsdcTokenAccount = testHelper.createTokenAccount(usdcTokenMint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda,offerBuyTokenPda, offerSellTokenPda } = testHelper.createOneTokenOfferAccounts(
            onreTokenMint, BigInt(0), 
            usdcTokenMint, BigInt(0), 
            boss, BigInt(200e9)
        );
        const offerUsdcTokenPda = offerBuyTokenPda;
        const offerOnreTokenPda = offerSellTokenPda;

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 10800; // 3 hours
        const priceFixDuration = 3600; // 1 hour

        // make offer
        await testHelper.makeOfferOne({
            offerId,
            buyTokenTotalAmount: 100e9,
            sellTokenStartAmount: 50e9,
            sellTokenEndAmount: 200e9,
            offerStartTime,
            offerEndTime,
            priceFixDuration,
            sellTokenMint: onreTokenMint,
            buyTokenMint: usdcTokenMint,
        })

        // time travel to last interval
        await testHelper.advanceClockBy((priceFixDuration * 3) - 1);

        // when
        await testHelper.takeOfferOne({
            sellTokenAmount: 100e9,
            offerPda,
            user,
        })

        // then
        await testHelper.expectTokenAccountAmountToBe(userOnreTokenAccount, BigInt(0));
        await testHelper.expectTokenAccountAmountToBe(userUsdcTokenAccount, BigInt(50e9));
        await testHelper.expectTokenAccountAmountToBe(offerUsdcTokenPda, BigInt(50e9));
        await testHelper.expectTokenAccountAmountToBe(offerOnreTokenPda, BigInt(100e9));
    });

    test("Take offer with minimum possible amount in first interval should succeed", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;

        const userOnreTokenAccount = testHelper.createTokenAccount(onreTokenMint, user.publicKey, BigInt(1), true);
        const userUsdcTokenAccount = testHelper.createTokenAccount(usdcTokenMint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda,offerBuyTokenPda, offerSellTokenPda } = testHelper.createOneTokenOfferAccounts(
            onreTokenMint, BigInt(0), 
            usdcTokenMint, BigInt(0), 
            boss, BigInt(1000e9)
        );
        const offerUsdcTokenPda = offerBuyTokenPda;
        const offerOnreTokenPda = offerSellTokenPda;

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 10800; // 3 hours
        const priceFixDuration = 3600; // 1 hour

        // make offer
        await testHelper.makeOfferOne({
            offerId,
            buyTokenTotalAmount: 1000e9,
            sellTokenStartAmount: 50e9,
            sellTokenEndAmount: 200e9,
            offerStartTime,
            offerEndTime,
            priceFixDuration,
            sellTokenMint: onreTokenMint,
            buyTokenMint: usdcTokenMint,
        })

        // when
        await testHelper.takeOfferOne({
            sellTokenAmount: 1,
            offerPda,
            user,
        })

        // then
        await testHelper.expectTokenAccountAmountToBe(userOnreTokenAccount, BigInt(0));
        await testHelper.expectTokenAccountAmountToBe(userUsdcTokenAccount, BigInt(10));
        await testHelper.expectTokenAccountAmountToBe(offerUsdcTokenPda, BigInt(999_999_999_990));
        await testHelper.expectTokenAccountAmountToBe(offerOnreTokenPda, BigInt(1));
    });

    test("Take offer with minimum possible amount in last interval should succeed", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;

        const userOnreTokenAccount = testHelper.createTokenAccount(onreTokenMint, user.publicKey, BigInt(1), true);
        const userUsdcTokenAccount = testHelper.createTokenAccount(usdcTokenMint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda,offerBuyTokenPda, offerSellTokenPda } = testHelper.createOneTokenOfferAccounts(
            onreTokenMint, BigInt(0), 
            usdcTokenMint, BigInt(0), 
            boss, BigInt(1000e9)
        );
        const offerUsdcTokenPda = offerBuyTokenPda;
        const offerOnreTokenPda = offerSellTokenPda;

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 10800; // 3 hours
        const priceFixDuration = 3600; // 1 hour

        // make offer
        await testHelper.makeOfferOne({
            offerId,
            buyTokenTotalAmount: 1000e9,
            sellTokenStartAmount: 50e9,
            sellTokenEndAmount: 200e9,
            offerStartTime,
            offerEndTime,
            priceFixDuration,
            sellTokenMint: onreTokenMint,
            buyTokenMint: usdcTokenMint,
        })

        // time travel to last interval
        await testHelper.advanceClockBy(priceFixDuration * 2.5);

        // when
        await testHelper.takeOfferOne({
            sellTokenAmount: 1,
            offerPda,
            user,
        })

        // then
        await testHelper.expectTokenAccountAmountToBe(userOnreTokenAccount, BigInt(0));
        await testHelper.expectTokenAccountAmountToBe(userUsdcTokenAccount, BigInt(5));
        await testHelper.expectTokenAccountAmountToBe(offerUsdcTokenPda, BigInt(999_999_999_995));
        await testHelper.expectTokenAccountAmountToBe(offerOnreTokenPda, BigInt(1));
    });

    test("Take offer with more sell token amount than available in offer should fail", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();

        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;

        testHelper.createTokenAccount(onreTokenMint, user.publicKey, BigInt(100e9), true);
        testHelper.createTokenAccount(usdcTokenMint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda } = testHelper.createOneTokenOfferAccounts( 
            onreTokenMint, BigInt(0), 
            usdcTokenMint, BigInt(0), 
            boss, BigInt(600e9)
        );
        
        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 25200; // 7 hours
        const priceFixDuration = 3600; // 1 hour

        // make offer
        await testHelper.makeOfferOne({
            offerId, 
            buyTokenTotalAmount: 100e9, 
            sellTokenStartAmount: 150e9, 
            sellTokenEndAmount: 500e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration,
            sellTokenMint: onreTokenMint, 
            buyTokenMint: usdcTokenMint, 
        })

        // when
        await expect(testHelper.takeOfferOne({
            sellTokenAmount: 101e9,
            offerPda,
            user,
        })).rejects.toThrow();
    })

    test("Take offer with two buy tokens in second interval should succeed", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();

        const userSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = testHelper.createTokenAccount(buyToken1Mint, user.publicKey, BigInt(0), true);
        const userBuyToken2Account = testHelper.createTokenAccount(buyToken2Mint, user.publicKey, BigInt(0), true);
        
        // create offer accounts
        const { offerId, offerPda, offerSellTokenPda, offerBuyToken1Pda, offerBuyToken2Pda } = testHelper.createTwoTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            buyToken2Mint, BigInt(0), 
            boss, BigInt(600e9), BigInt(6000e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 259200; // 3 days

        // make offer
        await testHelper.makeOfferTwo({
            offerId, 
            buyToken1TotalAmount: 100e9, 
            buyToken2TotalAmount: 1000e9, 
            sellTokenStartAmount: 100e9, 
            sellTokenEndAmount: 250e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration: 86400,
            sellTokenMint,
            buyToken1Mint,
            buyToken2Mint,
        })

        // time travel to next interval
        await testHelper.advanceClockBy(90000);

        // when
        await testHelper.takeOfferTwo({
            sellTokenAmount: 100e9,
            offerPda,
            user,
        })
            
        // then
        await testHelper.expectTokenAccountAmountToBe(userSellTokenAccount, BigInt(0));
        await testHelper.expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(100e9));
        await testHelper.expectTokenAccountAmountToBe(userBuyToken1Account, BigInt(50e9));
        await testHelper.expectTokenAccountAmountToBe(userBuyToken2Account, BigInt(500e9));
        await testHelper.expectTokenAccountAmountToBe(offerBuyToken1Pda, BigInt(50e9));
        await testHelper.expectTokenAccountAmountToBe(offerBuyToken2Pda, BigInt(500e9));
    });

    test("Taking an offer doesn't change the price", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;

        testHelper.createTokenAccount(onreTokenMint, user.publicKey, BigInt(100e9), true);
        testHelper.createTokenAccount(usdcTokenMint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda } = testHelper.createTwoTokenOfferAccounts( 
            onreTokenMint, BigInt(0), 
            usdcTokenMint, BigInt(0), 
            onreTokenMint, BigInt(0), 
            boss, BigInt(200e9), BigInt(100e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 10800; // 3 hours
        const priceFixDuration = 3600; // 1 hour

        // make offer
        await testHelper.makeOfferTwo({
            offerId, 
            buyToken1TotalAmount: 100e9, 
            buyToken2TotalAmount: 100e9, 
            sellTokenStartAmount: 150e9, 
            sellTokenEndAmount: 300e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration,
            sellTokenMint: onreTokenMint, 
            buyToken1Mint: usdcTokenMint, 
            buyToken2Mint: onreTokenMint, 
        })

        const offerAccountBefore = await testHelper.getOfferAccount(offerPda);
        expect(offerAccountBefore.buyToken1.amount.eq(new BN(100e9))).toBe(true);
        expect(offerAccountBefore.buyToken2.amount.eq(new BN(100e9))).toBe(true);
        expect(offerAccountBefore.sellTokenStartAmount.eq(new BN(150e9))).toBe(true);
        expect(offerAccountBefore.sellTokenEndAmount.eq(new BN(300e9))).toBe(true);

        // when
        await testHelper.takeOfferTwo({
            sellTokenAmount: 50e9,
            offerPda,
            user,
        })

        // time travel to last interval
        await testHelper.advanceClockBy(priceFixDuration * 1.5);

        const offerAccountAfter = await testHelper.getOfferAccount(offerPda);
        expect(offerAccountAfter.buyToken1.amount.eq(new BN(100e9))).toBe(true);
        expect(offerAccountAfter.buyToken2.amount.eq(new BN(100e9))).toBe(true);
        expect(offerAccountAfter.sellTokenStartAmount.eq(new BN(150e9))).toBe(true);
        expect(offerAccountAfter.sellTokenEndAmount.eq(new BN(300e9))).toBe(true);
    })

    test("Take offer after offer end time should fail", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const userSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = testHelper.createTokenAccount(buyToken1Mint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda, offerSellTokenPda, offerBuyTokenPda } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(600e9)
        );
        
        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 7200;

        // make offer
        await testHelper.makeOfferOne({
            offerId, 
            buyTokenTotalAmount: 100e9, 
            sellTokenStartAmount: 100e9, 
            sellTokenEndAmount: 200e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration: 3600,
            sellTokenMint,
            buyTokenMint: buyToken1Mint,
        })

        // time travel to after offer end time
        await testHelper.advanceClockBy(7201);

        // when
        await expect(testHelper.takeOfferOne({
            sellTokenAmount: 10e9,
            offerPda,
            user,
        })).rejects.toThrow(RegExp(".*InvalidCurrentTime.*"));

        // then
        await testHelper.expectTokenAccountAmountToBe(userSellTokenAccount, BigInt(100e9));
        await testHelper.expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(0));
        await testHelper.expectTokenAccountAmountToBe(userBuyToken1Account, BigInt(0));
        await testHelper.expectTokenAccountAmountToBe(offerBuyTokenPda, BigInt(100e9));
    })

    test("Take offer before offer start time should fail", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const userSellTokenAccount = testHelper.createTokenAccount(sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = testHelper.createTokenAccount(buyToken1Mint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda, offerSellTokenPda, offerBuyTokenPda } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0), 
            buyToken1Mint, BigInt(0), 
            boss, BigInt(600e9)
        );

        const offerStartTime = await testHelper.getCurrentClockTime() + 1;
        const offerEndTime = offerStartTime + 7200;

        // make offer
        await testHelper.makeOfferOne({
            offerId, 
            buyTokenTotalAmount: 100e9, 
            sellTokenStartAmount: 100e9, 
            sellTokenEndAmount: 200e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration: 3600,
            sellTokenMint,
            buyTokenMint: buyToken1Mint,
        })

        // when
        await expect(testHelper.takeOfferOne({
            sellTokenAmount: 10e9,
            offerPda,
            user,
        })).rejects.toThrow(RegExp(".*InvalidCurrentTime.*"));

        // then
        await testHelper.expectTokenAccountAmountToBe(userSellTokenAccount, BigInt(100e9));
        await testHelper.expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(0));
        await testHelper.expectTokenAccountAmountToBe(userBuyToken1Account, BigInt(0));
        await testHelper.expectTokenAccountAmountToBe(offerBuyTokenPda, BigInt(100e9));
    });

    test("Take offer with the same buy token 2 as sell token should succeed", async () => {
        // given
        // create user
        const user = testHelper.createUserAccount();
        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;

        const userOnreTokenAccount = testHelper.createTokenAccount(onreTokenMint, user.publicKey, BigInt(100e9), true);
        const userUsdcTokenAccount = testHelper.createTokenAccount(usdcTokenMint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const { offerId, offerPda, offerBuyToken1Pda, offerBuyToken2Pda } = testHelper.createTwoTokenOfferAccounts(
            onreTokenMint, BigInt(0), 
            usdcTokenMint, BigInt(0), 
            onreTokenMint, BigInt(0), 
            boss, BigInt(200e9), BigInt(100e9)
        );
        const offerUsdcTokenPda = offerBuyToken1Pda;
        const offerOnreTokenPda = offerBuyToken2Pda;

        const offerStartTime = await testHelper.getCurrentClockTime();
        const offerEndTime = offerStartTime + 25200; // 7 hours
        const priceFixDuration = 3600; // 1 hour

        // make offer
        await testHelper.makeOfferTwo({
            offerId, 
            buyToken1TotalAmount: 100e9, 
            buyToken2TotalAmount: 100e9, 
            sellTokenStartAmount: 150e9, 
            sellTokenEndAmount: 500e9, 
            offerStartTime, 
            offerEndTime, 
            priceFixDuration,
            sellTokenMint: onreTokenMint, 
            buyToken1Mint: usdcTokenMint, 
            buyToken2Mint: onreTokenMint, 
        })

        // when
        // first interval: 
        // sell token amount: 200e9
        // users can exchange 1 ONRE = 0.5 USDC + 0.5 ONRE 
        await testHelper.takeOfferTwo({
            sellTokenAmount: 10e9,
            offerPda,
            user,
        })

        // then
        await testHelper.expectTokenAccountAmountToBe(userOnreTokenAccount, BigInt(95e9)); // 100 - 10 + 5
        await testHelper.expectTokenAccountAmountToBe(userUsdcTokenAccount, BigInt(5e9)); // 0 + 5
        await testHelper.expectTokenAccountAmountToBe(offerOnreTokenPda, BigInt(105e9)); // 100 + 10 - 5
        await testHelper.expectTokenAccountAmountToBe(offerUsdcTokenPda, BigInt(95e9)); // 100 - 5

        // time travel to last interval
        await testHelper.advanceClockBy(priceFixDuration * 6.5);

        // last interval: 
        // sell token amount: 500e9
        // users can exchange 1 ONRE = 0.2 USDC + 0.2 ONRE
        await testHelper.takeOfferTwo({
            sellTokenAmount: 10e9,
            offerPda,
            user,
        })

        // then
        await testHelper.expectTokenAccountAmountToBe(userOnreTokenAccount, BigInt(87e9)); // 95 - 10 + 2
        await testHelper.expectTokenAccountAmountToBe(userUsdcTokenAccount, BigInt(7e9)); // 5 + 2
        await testHelper.expectTokenAccountAmountToBe(offerOnreTokenPda, BigInt(113e9)); // 105 + 10 - 2
        await testHelper.expectTokenAccountAmountToBe(offerUsdcTokenPda, BigInt(93e9)); // 95 - 2
    });

    test("should allow taking offer until empty", async () => {
        // given

        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;

        const user1 = testHelper.createUserAccount();
        const user1OnreTokenAccount = testHelper.createTokenAccount(onreTokenMint, user1.publicKey, BigInt(100e9), true);
        const user1UsdcTokenAccount = testHelper.createTokenAccount(usdcTokenMint, user1.publicKey, BigInt(0), true);

        const user2 = testHelper.createUserAccount();
        const user2OnreTokenAccount = testHelper.createTokenAccount(sellTokenMint, user2.publicKey, BigInt(100e9), true);
        const user2UsdcTokenAccount = testHelper.createTokenAccount(buyToken1Mint, user2.publicKey, BigInt(0), true);

        const { offerId, offerPda, offerSellTokenPda, offerBuyTokenPda } = testHelper.createOneTokenOfferAccounts(
            onreTokenMint, BigInt(0),
            usdcTokenMint, BigInt(0),
            boss, BigInt(1000e9)
        );

        const offerUsdcTokenPda = offerBuyTokenPda;
        const offerOnreTokenPda = offerSellTokenPda;

        const offerStartTime = await testHelper.getCurrentClockTime();
        const priceFixDuration = 3600; // 1 hour
        const offerEndTime = offerStartTime + (13 * priceFixDuration);

        // make offer
        await testHelper.makeOfferOne({
            offerId,
            buyTokenTotalAmount: 120e9,
            sellTokenStartAmount: 50e9,
            sellTokenEndAmount: 180e9,
            offerStartTime,
            offerEndTime,
            priceFixDuration,
            sellTokenMint: onreTokenMint,
            buyTokenMint: usdcTokenMint,
        })

        // when
        // first interval:
        // sell token amount: 60e9
        // user1 gives sell tokens: 60e9 
        // user1 receives buy tokens: 30e9
        await testHelper.takeOfferOne({
            sellTokenAmount: 30e9,
            offerPda,
            user: user1,
        })

        // then
        await testHelper.expectTokenAccountAmountToBe(user1OnreTokenAccount, BigInt(70e9));
        await testHelper.expectTokenAccountAmountToBe(user1UsdcTokenAccount, BigInt(60e9));
        await testHelper.expectTokenAccountAmountToBe(offerOnreTokenPda, BigInt(30e9));
        await testHelper.expectTokenAccountAmountToBe(offerUsdcTokenPda, BigInt(60e9));

        // time travel to last interval
        await testHelper.advanceClockBy(priceFixDuration * 12.5);

        // last interval:
        // sell token amount: 180e9
        // user2 gives sell tokens: 120e9
        // user2 receives buy tokens: 60e9
        await testHelper.takeOfferOne({
            sellTokenAmount: 90e9,
            offerPda,
            user: user2,
        })

        // then
        await testHelper.expectTokenAccountAmountToBe(user2OnreTokenAccount, BigInt(10e9));
        await testHelper.expectTokenAccountAmountToBe(user2UsdcTokenAccount, BigInt(60e9));
        await testHelper.expectTokenAccountAmountToBe(offerOnreTokenPda, BigInt(120e9));
        await testHelper.expectTokenAccountAmountToBe(offerUsdcTokenPda, BigInt(0));
    });

    test("take offer in last seconds should fail", async () => {
        // given
        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;
        
        const user = testHelper.createUserAccount();
        const userOnreTokenAccount = testHelper.createTokenAccount(onreTokenMint, user.publicKey, BigInt(300e9), true);
        const userUsdcTokenAccount = testHelper.createTokenAccount(usdcTokenMint, user.publicKey, BigInt(0), true);
        
        const { offerId, offerPda, offerBuyTokenPda, offerSellTokenPda } = testHelper.createOneTokenOfferAccounts(
            onreTokenMint, BigInt(0),
            usdcTokenMint, BigInt(0),
            boss, BigInt(1000e9)
        );

        const offerUsdcTokenPda = offerBuyTokenPda;
        const offerOnreTokenPda = offerSellTokenPda;

        const offerStartTime = await testHelper.getCurrentClockTime();
        const priceFixDuration = 3600; // 1 hour
        const offerEndTime = offerStartTime + (2 * priceFixDuration);

        // make offer
        await testHelper.makeOfferOne({
            offerId,
            buyTokenTotalAmount: 100e9,
            sellTokenStartAmount: 100e9,
            sellTokenEndAmount: 300e9,
            offerStartTime,
            offerEndTime,
            priceFixDuration,
            sellTokenMint: onreTokenMint,
            buyTokenMint: usdcTokenMint,
        })

        // time travel to last second
        await testHelper.advanceClockBy(priceFixDuration * 2);

        // when
        await expect(testHelper.takeOfferOne({
            sellTokenAmount: 300e9,
            offerPda,
            user: user,
        })).rejects.toThrow(RegExp(".*InvalidCurrentTime.*"));
    })
})