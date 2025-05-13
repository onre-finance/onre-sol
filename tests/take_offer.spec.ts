import { BanksClient, ProgramTestContext, AddedProgram, startAnchor, Clock } from "solana-bankrun";
import { BankrunProvider } from "anchor-bankrun";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { OnreApp } from "../target/types/onre_app";
import idl from "../target/idl/onre_app.json";
import { INITIAL_LAMPORTS, ONREAPP_PROGRAM_ID } from "./test_constants";
import { createTokenAccount, createMint } from "./test_helper";
import * as anchor from '@coral-xyz/anchor';
import * as borsh from 'borsh';
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import { AccountLayout } from "@solana/spl-token";

describe("take offer", () => {
    let context: ProgramTestContext;
    let provider: BankrunProvider;
    let program: Program<OnreApp>;
    let sellTokenMint: PublicKey;
    let buyToken1Mint: PublicKey;
    let buyToken2Mint: PublicKey;
    let boss: PublicKey;
    let statePda: PublicKey;
    let client: BanksClient;

    beforeAll(async () => {
        const programInfo: AddedProgram = {
            programId: ONREAPP_PROGRAM_ID,
            name: "onreapp",
        };

        context = await startAnchor(".", [programInfo], []);

        provider = new BankrunProvider(context);
        program = new Program(
            idl,
            provider,
        );

        client = context.banksClient;

        [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], ONREAPP_PROGRAM_ID);    
        boss = provider.wallet.publicKey;
        
        // Create mints
        sellTokenMint = createMint(context, boss, BigInt(100_000e9), 9);
        buyToken1Mint = createMint(context, boss, BigInt(100_000e9), 9);
        buyToken2Mint = createMint(context, boss, BigInt(100_000e9), 9);
        
        await program.methods.initialize().accounts({ boss }).rpc();
    });

    test("Take offer with one buy token in first interval", async () => {
        // given
        // create user
        const user = Keypair.generate();
        context.setAccount(user.publicKey, {
            executable: false,
            data: new Uint8Array([]),
            lamports: INITIAL_LAMPORTS,
            owner: SystemProgram.programId,
        });
        const userSellTokenAccount = createTokenAccount(context, sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = createTokenAccount(context, buyToken1Mint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const offerId = new anchor.BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerSellTokenPda = createTokenAccount(context, sellTokenMint, offerAuthority, BigInt(0), true);
        const offerBuyTokenPda = createTokenAccount(context, buyToken1Mint, offerAuthority, BigInt(0), true);
        const bossBuyTokenAccount1 = createTokenAccount(context, buyToken1Mint, boss, BigInt(600e9));
        const offerStartTime = Number((await client.getClock()).unixTimestamp);
        const offerEndTime = offerStartTime + 7200;

        // make offer
        await program.methods
            .makeOfferOne(
                offerId, new anchor.BN(100e9), 
                new anchor.BN(100e9), new anchor.BN(200e9),
                new anchor.BN(offerStartTime), new anchor.BN(offerEndTime), new anchor.BN(3600))
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .rpc();

        // when
        await program.methods
            .takeOfferOne(new anchor.BN(10e9))
            .accounts({ offer: offerPda, user: user.publicKey })
            .signers([user])
            .rpc();

        // then
        expectTokenAccountAmountToBe(userSellTokenAccount, BigInt(90e9));
        expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(10e9));
        expectTokenAccountAmountToBe(userBuyToken1Account, BigInt(6666666666));
        expectTokenAccountAmountToBe(offerBuyTokenPda, BigInt(93333333334));
    });

    test("Take offer with one buy token in second interval", async () => {
        // given
        // create user
        const user = Keypair.generate();
        context.setAccount(user.publicKey, {
            executable: false,
            data: new Uint8Array([]),
            lamports: INITIAL_LAMPORTS,
            owner: SystemProgram.programId,
        });
        const userSellTokenAccount = createTokenAccount(context, sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = createTokenAccount(context, buyToken1Mint, user.publicKey, BigInt(0), true);
        
        // create offer accounts
        const offerId = new anchor.BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerSellTokenPda = createTokenAccount(context, sellTokenMint, offerAuthority, BigInt(0), true);
        const offerBuyTokenPda = createTokenAccount(context, buyToken1Mint, offerAuthority, BigInt(0), true);
        const bossBuyTokenAccount1 = createTokenAccount(context, buyToken1Mint, boss, BigInt(600e9));
        const offerStartTime = Number((await client.getClock()).unixTimestamp);
        const offerEndTime = offerStartTime + 7200;

        // make offer
        await program.methods
            .makeOfferOne(
                offerId, new anchor.BN(100e9), 
                new anchor.BN(100e9), new anchor.BN(200e9), 
                new anchor.BN(offerStartTime), new anchor.BN(offerEndTime), new anchor.BN(3600))
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .rpc();

        // time travel to next interval
        const currentClock = await client.getClock();
        context.setClock(
            new Clock(
                currentClock.slot,
                currentClock.epochStartTimestamp,
                currentClock.epoch,
                currentClock.leaderScheduleEpoch,
                currentClock.unixTimestamp + BigInt(3600),
            )
        )

        // when
        await program.methods
            .takeOfferOne(new anchor.BN(10e9))
            .accounts({ offer: offerPda, user: user.publicKey })
            .signers([user])
            .rpc();
            
        // then
        expectTokenAccountAmountToBe(userSellTokenAccount, BigInt(90e9));
        expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(10e9));
        expectTokenAccountAmountToBe(userBuyToken1Account, BigInt(5e9));
        expectTokenAccountAmountToBe(offerBuyTokenPda, BigInt(95e9));
    });

    test("Take offer with two buy tokens in second interval", async () => {
        // given
        // create user
        const user = Keypair.generate();
        context.setAccount(user.publicKey, {
            executable: false,
            data: new Uint8Array([]),
            lamports: INITIAL_LAMPORTS,
            owner: SystemProgram.programId,
        });
        const userSellTokenAccount = createTokenAccount(context, sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = createTokenAccount(context, buyToken1Mint, user.publicKey, BigInt(0), true);
        const userBuyToken2Account = createTokenAccount(context, buyToken2Mint, user.publicKey, BigInt(0), true);
        
        // create offer accounts
        const offerId = new anchor.BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerSellTokenPda = createTokenAccount(context, sellTokenMint, offerAuthority, BigInt(0), true);
        const offerBuyToken1Pda = createTokenAccount(context, buyToken1Mint, offerAuthority, BigInt(0), true);
        const offerBuyToken2Pda = createTokenAccount(context, buyToken2Mint, offerAuthority, BigInt(0), true);
        const bossBuyTokenAccount1 = createTokenAccount(context, buyToken1Mint, boss, BigInt(600e9));
        const bossBuyTokenAccount2 = createTokenAccount(context, buyToken2Mint, boss, BigInt(6000e9));
        const offerStartTime = Number((await client.getClock()).unixTimestamp);
        const offerEndTime = offerStartTime + 259200; // 3 days

        // make offer
        await program.methods
            .makeOfferTwo(
                offerId, new anchor.BN(100e9), new anchor.BN(1000e9),
                new anchor.BN(100e9), new anchor.BN(250e9), 
                new anchor.BN(offerStartTime), new anchor.BN(offerEndTime), new anchor.BN(86400))
            .accounts({ sellTokenMint, buyToken1Mint, buyToken2Mint, state: statePda })
            .rpc();

        // time travel to next interval
        const currentClock = await client.getClock();
        context.setClock(
            new Clock(
                currentClock.slot,
                currentClock.epochStartTimestamp,
                currentClock.epoch,
                currentClock.leaderScheduleEpoch,
                currentClock.unixTimestamp + BigInt(90000),
            )
        )

        // when
        await program.methods
            .takeOfferTwo(new anchor.BN(100e9))
            .accounts({ offer: offerPda, user: user.publicKey })
            .signers([user])
            .rpc();
            
        // then
        expectTokenAccountAmountToBe(userSellTokenAccount, BigInt(0));
        expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(100e9));
        expectTokenAccountAmountToBe(userBuyToken1Account, BigInt(50e9));
        expectTokenAccountAmountToBe(userBuyToken2Account, BigInt(500e9));
        expectTokenAccountAmountToBe(offerBuyToken1Pda, BigInt(50e9));
        expectTokenAccountAmountToBe(offerBuyToken2Pda, BigInt(500e9));
    });

    test("Taking an offer doesn't change the price", async () => {
        // given
        // create user
        const user = Keypair.generate();
        context.setAccount(user.publicKey, {
            executable: false,
            data: new Uint8Array([]),
            lamports: INITIAL_LAMPORTS,
            owner: SystemProgram.programId,
        });
        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;

        const userOnreTokenAccount = createTokenAccount(context, onreTokenMint, user.publicKey, BigInt(100e9), true);
        const userUsdcTokenAccount = createTokenAccount(context, usdcTokenMint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const offerId = new anchor.BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerOnreTokenPda = createTokenAccount(context, onreTokenMint, offerAuthority, BigInt(0), true);
        const offerUsdcTokenPda = createTokenAccount(context, usdcTokenMint, offerAuthority, BigInt(0), true);
        const bossUsdcTokenAccount = createTokenAccount(context, buyToken1Mint, boss, BigInt(200e9));
        const bossOnreTokenAccount = createTokenAccount(context, onreTokenMint, boss, BigInt(100e9));

        const offerStartTime = Number((await client.getClock()).unixTimestamp);
        const offerEndTime = offerStartTime + 10800; // 3 hours
        const priceFixDuration = 3600; // 1 hour

        // make offer
        await program.methods
            .makeOfferTwo(
                offerId, 
                new anchor.BN(100e9), // buy usdc token amount
                new anchor.BN(100e9), // buy onre token amount
                new anchor.BN(150e9), // sell onre token amount start
                new anchor.BN(300e9), // sell onre token amount end
                new anchor.BN(offerStartTime), 
                new anchor.BN(offerEndTime), 
                new anchor.BN(priceFixDuration))
            .accounts({ 
                sellTokenMint: onreTokenMint, 
                buyToken1Mint: usdcTokenMint, 
                buyToken2Mint: onreTokenMint, 
                state: statePda })
            .rpc();

        const offerAccountBefore = await program.account.offer.fetch(offerPda);
        expect(offerAccountBefore.buyToken1.amount.eq(new anchor.BN(100e9))).toBe(true);
        expect(offerAccountBefore.buyToken2.amount.eq(new anchor.BN(100e9))).toBe(true);
        expect(offerAccountBefore.sellTokenStartAmount.eq(new anchor.BN(150e9))).toBe(true);
        expect(offerAccountBefore.sellTokenEndAmount.eq(new anchor.BN(300e9))).toBe(true);

        // when
        await program.methods
            .takeOfferTwo(new anchor.BN(50e9))
            .accounts({ offer: offerPda, user: user.publicKey })
            .signers([user])
            .rpc();

        // time travel to last interval
        const currentClock = await client.getClock();
        context.setClock(
            new Clock(
                currentClock.slot,
                currentClock.epochStartTimestamp,
                currentClock.epoch,
                currentClock.leaderScheduleEpoch,
                currentClock.unixTimestamp + BigInt(priceFixDuration * 1.5),
            )
        )

        const offerAccountAfter = await program.account.offer.fetch(offerPda);
        expect(offerAccountAfter.buyToken1.amount.eq(new anchor.BN(100e9))).toBe(true);
        expect(offerAccountAfter.buyToken2.amount.eq(new anchor.BN(100e9))).toBe(true);
        expect(offerAccountAfter.sellTokenStartAmount.eq(new anchor.BN(150e9))).toBe(true);
        expect(offerAccountAfter.sellTokenEndAmount.eq(new anchor.BN(300e9))).toBe(true);
    })

    test("Take offer after offer end time should fail", async () => {
        // given
        // create user
        const user = Keypair.generate();
        context.setAccount(user.publicKey, {
            executable: false,
            data: new Uint8Array([]),
            lamports: INITIAL_LAMPORTS,
            owner: SystemProgram.programId,
        });
        const userSellTokenAccount = createTokenAccount(context, sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = createTokenAccount(context, buyToken1Mint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const offerId = new anchor.BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerSellTokenPda = createTokenAccount(context, sellTokenMint, offerAuthority, BigInt(0), true);
        const offerBuyTokenPda = createTokenAccount(context, buyToken1Mint, offerAuthority, BigInt(0), true);
        const bossBuyTokenAccount1 = createTokenAccount(context, buyToken1Mint, boss, BigInt(600e9));
        const offerStartTime = Number((await client.getClock()).unixTimestamp);
        const offerEndTime = offerStartTime + 7200;

        // make offer
        await program.methods
            .makeOfferOne(
                offerId, new anchor.BN(100e9), 
                new anchor.BN(100e9), new anchor.BN(200e9), 
                new anchor.BN(offerStartTime), new anchor.BN(offerEndTime), new anchor.BN(3600))
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .rpc();

        // time travel to after offer end time
        const currentClock = await client.getClock();
        context.setClock(
            new Clock(
                currentClock.slot,
                currentClock.epochStartTimestamp,
                currentClock.epoch,
                currentClock.leaderScheduleEpoch,
                currentClock.unixTimestamp + BigInt(7500),
            )
        )

        // when
        await expect(program.methods
            .takeOfferOne(new anchor.BN(10e9))
            .accounts({ offer: offerPda, user: user.publicKey })
            .signers([user])
            .rpc()).rejects.toThrow(RegExp(".*InvalidCurrentTime.*"));

        // then
        expectTokenAccountAmountToBe(userSellTokenAccount, BigInt(100e9));
        expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(0));
        expectTokenAccountAmountToBe(userBuyToken1Account, BigInt(0));
        expectTokenAccountAmountToBe(offerBuyTokenPda, BigInt(100e9));
    })

    test("Take offer before offer start time should fail", async () => {
        // given
        // create user
        const user = Keypair.generate();
        context.setAccount(user.publicKey, {
            executable: false,
            data: new Uint8Array([]),
            lamports: INITIAL_LAMPORTS,
            owner: SystemProgram.programId,
        });
        const userSellTokenAccount = createTokenAccount(context, sellTokenMint, user.publicKey, BigInt(100e9), true);
        const userBuyToken1Account = createTokenAccount(context, buyToken1Mint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const offerId = new anchor.BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerSellTokenPda = createTokenAccount(context, sellTokenMint, offerAuthority, BigInt(0), true);
        const offerBuyTokenPda = createTokenAccount(context, buyToken1Mint, offerAuthority, BigInt(0), true);
        const bossBuyTokenAccount1 = createTokenAccount(context, buyToken1Mint, boss, BigInt(600e9));
        const offerStartTime = (Number((await client.getClock()).unixTimestamp)) + 1000;
        const offerEndTime = offerStartTime + 7200;

        // make offer
        await program.methods
            .makeOfferOne(
                offerId, new anchor.BN(100e9), 
                new anchor.BN(100e9), new anchor.BN(200e9), 
                new anchor.BN(offerStartTime), new anchor.BN(offerEndTime), new anchor.BN(3600))
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .rpc();

        // when
        await expect(program.methods
            .takeOfferOne(new anchor.BN(10e9))
            .accounts({ offer: offerPda, user: user.publicKey })
            .signers([user])
            .rpc()).rejects.toThrow(RegExp(".*InvalidCurrentTime.*"));

        // then
        expectTokenAccountAmountToBe(userSellTokenAccount, BigInt(100e9));
        expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(0));
        expectTokenAccountAmountToBe(userBuyToken1Account, BigInt(0));
        expectTokenAccountAmountToBe(offerBuyTokenPda, BigInt(100e9));
    });

    test("Take offer with two buy tokens should keep the same price for second token", async () => {
        // given
        // create user
        const user = Keypair.generate();
        context.setAccount(user.publicKey, {
            executable: false,
            data: new Uint8Array([]),
            lamports: INITIAL_LAMPORTS,
            owner: SystemProgram.programId,
        });
        const onreTokenMint = sellTokenMint;
        const usdcTokenMint = buyToken1Mint;

        const userOnreTokenAccount = createTokenAccount(context, onreTokenMint, user.publicKey, BigInt(100e9), true);
        const userUsdcTokenAccount = createTokenAccount(context, usdcTokenMint, user.publicKey, BigInt(0), true);

        // create offer accounts
        const offerId = new anchor.BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerOnreTokenPda = createTokenAccount(context, onreTokenMint, offerAuthority, BigInt(0), true);
        const offerUsdcTokenPda = createTokenAccount(context, usdcTokenMint, offerAuthority, BigInt(0), true);
        const bossUsdcTokenAccount = createTokenAccount(context, buyToken1Mint, boss, BigInt(200e9));
        const bossOnreTokenAccount = createTokenAccount(context, onreTokenMint, boss, BigInt(100e9));

        const offerStartTime = Number((await client.getClock()).unixTimestamp);
        const offerEndTime = offerStartTime + 25200; // 7 hours
        const priceFixDuration = 3600; // 1 hour

        // make offer
        await program.methods
            .makeOfferTwo(
                offerId, 
                new anchor.BN(100e9), // buy usdc token amount
                new anchor.BN(100e9), // buy onre token amount
                new anchor.BN(150e9), // sell onre token amount start
                new anchor.BN(500e9), // sell onre token amount end
                new anchor.BN(offerStartTime), 
                new anchor.BN(offerEndTime), 
                new anchor.BN(priceFixDuration))
            .accounts({ 
                sellTokenMint: onreTokenMint, 
                buyToken1Mint: usdcTokenMint, 
                buyToken2Mint: onreTokenMint, 
                state: statePda })
            .rpc();

        // when
        // first interval: 
        // sell token amount: 200e9
        // users can exchange 1 ONRE = 0.5 USDC + 0.5 ONRE 
        await program.methods
            .takeOfferTwo(new anchor.BN(10e9))
            .accounts({ offer: offerPda, user: user.publicKey })
            .signers([user])
            .rpc();

        // then
        await expectTokenAccountAmountToBe(userOnreTokenAccount, BigInt(95e9)); // 100 - 10 + 5
        await expectTokenAccountAmountToBe(userUsdcTokenAccount, BigInt(5e9)); // 0 + 5
        await expectTokenAccountAmountToBe(offerOnreTokenPda, BigInt(105e9)); // 100 + 10 - 5
        await expectTokenAccountAmountToBe(offerUsdcTokenPda, BigInt(95e9)); // 100 - 5

        // time travel to last interval
        const currentClock = await client.getClock();
        context.setClock(
            new Clock(
                currentClock.slot,
                currentClock.epochStartTimestamp,
                currentClock.epoch,
                currentClock.leaderScheduleEpoch,
                currentClock.unixTimestamp + BigInt(priceFixDuration * 6.5),
            )
        )

        // last interval: 
        // sell token amount: 500e9
        // users can exchange 1 ONRE = 0.2 USDC + 0.2 ONRE
        await program.methods
            .takeOfferTwo(new anchor.BN(10e9))
            .accounts({ offer: offerPda, user: user.publicKey })
            .signers([user])
            .rpc();

        // then
        expectTokenAccountAmountToBe(userOnreTokenAccount, BigInt(87e9)); // 95 - 10 + 2
        expectTokenAccountAmountToBe(userUsdcTokenAccount, BigInt(7e9)); // 5 + 2
        expectTokenAccountAmountToBe(offerOnreTokenPda, BigInt(113e9)); // 105 + 10 - 2
        expectTokenAccountAmountToBe(offerUsdcTokenPda, BigInt(93e9)); // 95 - 2
    })

    async function expectTokenAccountAmountToBe(tokenAccount: PublicKey, amount: bigint) {
        const account = await client.getAccount(tokenAccount);
        const tokenAccountData = AccountLayout.decode(account.data)
        expect(tokenAccountData.amount).toBe(amount)
    }
})