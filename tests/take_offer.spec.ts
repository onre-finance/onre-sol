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

    async function createOfferWith1BuyTokenAccount(
        sellTokenStartAmount: number,
        sellTokenEndAmount: number,
        buyToken1Amount: number,
        priceFixDuration: number,
        offerStartTime: number,
        offerEndTime: number,
    ): Promise<PublicKey> {
        const offerId = new anchor.BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerSellTokenPda = createTokenAccount(context, sellTokenMint, offerAuthority, BigInt(0), true);
        const offerBuyTokenPda = createTokenAccount(context, buyToken1Mint, offerAuthority, BigInt(0), true);
        const bossBuyTokenAccount1 = createTokenAccount(context, buyToken1Mint, boss, BigInt(600e9));

        await program.methods
        .makeOfferOne(
            offerId, new anchor.BN(buyToken1Amount), 
            new anchor.BN(sellTokenStartAmount), new anchor.BN(sellTokenEndAmount), 
            new anchor.BN(offerStartTime), new anchor.BN(offerEndTime), new anchor.BN(priceFixDuration))
        .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
        .rpc();

        return offerPda;
    }

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

        const offerAccount = await program.account.offer.fetch(offerPda);

        // when
        await program.methods
            .takeOfferOne(new anchor.BN(10e9))
            .accounts({ offer: offerPda, user: user.publicKey })
            .signers([user])
            .rpc();

        // then
        const userSellTokenAccountData = AccountLayout.decode((await client.getAccount(userSellTokenAccount)).data)
        expect(userSellTokenAccountData.amount).toBe(BigInt(90e9));

        const offerSellTokenAccountData = AccountLayout.decode((await client.getAccount(offerSellTokenPda)).data)
        expect(offerSellTokenAccountData.amount).toBe(BigInt(10e9));

        const userBuyToken1AccountData = AccountLayout.decode((await client.getAccount(userBuyToken1Account)).data)
        expect(userBuyToken1AccountData.amount).toBe(BigInt(6666666666));

        const offerBuyToken1AccountData = AccountLayout.decode((await client.getAccount(offerBuyTokenPda)).data)
        expect(offerBuyToken1AccountData.amount).toBe(BigInt(93333333334));
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
        const userSellTokenAccountData = AccountLayout.decode((await client.getAccount(userSellTokenAccount)).data)
        expect(userSellTokenAccountData.amount).toBe(BigInt(90e9));

        const offerSellTokenAccountData = AccountLayout.decode((await client.getAccount(offerSellTokenPda)).data)
        expect(offerSellTokenAccountData.amount).toBe(BigInt(10e9));

        const userBuyToken1AccountData = AccountLayout.decode((await client.getAccount(userBuyToken1Account)).data)
        expect(userBuyToken1AccountData.amount).toBe(BigInt(5e9));

        const offerBuyToken1AccountData = AccountLayout.decode((await client.getAccount(offerBuyTokenPda)).data)
        expect(offerBuyToken1AccountData.amount).toBe(BigInt(95e9));
    });
})