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
        const offerStartTime = Date.now() / 1000;
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
        const offerStartTime = Date.now() / 1000;
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
        const offerStartTime = Date.now() / 1000;
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
        const offerStartTime = Date.now() / 1000;
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
            .rpc()).rejects.toThrow();

        // then
        expectTokenAccountAmountToBe(userSellTokenAccount, BigInt(100e9));
        expectTokenAccountAmountToBe(offerSellTokenPda, BigInt(0));
        expectTokenAccountAmountToBe(userBuyToken1Account, BigInt(0));
        expectTokenAccountAmountToBe(offerBuyTokenPda, BigInt(100e9));
    })

    function expectTokenAccountAmountToBe(tokenAccount: PublicKey, amount: bigint) {
        client.getAccount(tokenAccount).then(account => {
            const tokenAccountData = AccountLayout.decode(account.data)
            return expect(tokenAccountData.amount).toBe(amount)
        })
    }
})