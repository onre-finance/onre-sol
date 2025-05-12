import { AddedAccount, AddedProgram, BanksClient, ProgramTestContext, startAnchor } from "solana-bankrun";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { Program } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { OnreApp } from "../target/types/onre_app";
import idl from "../target/idl/onre_app.json";
import * as anchor from '@coral-xyz/anchor';
import { ACCOUNT_SIZE, AccountLayout, ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccount, createAssociatedTokenAccountInstruction, getAssociatedTokenAddress, getAssociatedTokenAddressSync, MINT_SIZE, MintLayout, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import { ONREAPP_PROGRAM_ID } from "./test_constants";
import { createTokenAccount, createMint } from "./test_helper";

describe("make offer", () => {
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

        // Calculate all PDAs first
        [statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], ONREAPP_PROGRAM_ID);
        // [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        // [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
    
        boss = provider.wallet.publicKey;
        
        // Create mints
        sellTokenMint = createMint(context, boss, BigInt(100_000e9), 9);
        buyToken1Mint = createMint(context, boss, BigInt(100_000e9), 9);
        buyToken2Mint = createMint(context, boss, BigInt(100_000e9), 9);
        
        await program.methods.initialize().accounts({ boss }).rpc();
    });

    test("Make an offer", async () => {
        // given
        const offerId = new anchor.BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerSellTokenPda = createTokenAccount(context, sellTokenMint, offerAuthority, BigInt(0), true);
        const offerBuyTokenPda = createTokenAccount(context, buyToken1Mint, offerAuthority, BigInt(0), true);
        const offerStartTime = Date.now() / 1000;
        const offerEndTime = offerStartTime + 7200;
        const bossBuyTokenAccount1 = createTokenAccount(context, buyToken1Mint, boss, BigInt(600e9));

        // when
        await program.methods
            .makeOfferOne(
                offerId, new anchor.BN(500e9), 
                new anchor.BN(200e9), new anchor.BN(400e9), 
                new anchor.BN(offerStartTime), new anchor.BN(offerEndTime), new anchor.BN(3600))
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .rpc();

        // then
        const offerAccount = await program.account.offer.fetch(offerPda);
        expect(offerAccount.offerId.eq(offerId)).toBe(true);
        // sell token
        expect(offerAccount.sellTokenStartAmount.eq(new anchor.BN(200e9))).toBe(true);
        expect(offerAccount.sellTokenEndAmount.eq(new anchor.BN(400e9))).toBe(true);
        expect(offerAccount.sellTokenMint.toBase58()).toEqual(sellTokenMint.toBase58());
        // buy token
        expect(offerAccount.buyToken1.amount.eq(new anchor.BN(500e9))).toBe(true);
        expect(offerAccount.buyToken1.mint.toBase58()).toEqual(buyToken1Mint.toBase58());
        expect(offerAccount.buyToken2.amount.eq(new anchor.BN(0))).toBe(true);
        expect(offerAccount.buyToken2.mint.toBase58()).toEqual(SYSTEM_PROGRAM_ID.toBase58());
        // offer
        expect(offerAccount.priceFixDuration.eq(new anchor.BN(3600))).toBe(true);
        expect(offerAccount.offerStartTime.eq(new anchor.BN(offerStartTime))).toBe(true);
        expect(offerAccount.offerEndTime.eq(new anchor.BN(offerEndTime))).toBe(true);
        
        // 500 tokens substracted from boss buy token account
        const bossBuyTokenAccountData = AccountLayout.decode((await client.getAccount(bossBuyTokenAccount1)).data)
        expect(bossBuyTokenAccountData.amount).toBe(BigInt(100e9));

        // 500 tokens added to offer buy token account
        const offerBuyTokenAccountData = AccountLayout.decode((await client.getAccount(offerBuyTokenPda)).data)
        expect(offerBuyTokenAccountData.amount).toBe(BigInt(500e9));

        // offerSellTokenPda stays empty
        const offerSellTokenAccountData = AccountLayout.decode((await client.getAccount(offerSellTokenPda)).data)
        expect(offerSellTokenAccountData.amount).toBe(BigInt(0));
    });

    test("Make offer fails when price_fix_duration higher than offer duration", async () => {
        // given
        const offerId = new anchor.BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerSellTokenPda = createTokenAccount(context, sellTokenMint, offerAuthority, BigInt(0), true);
        const offerBuyTokenPda = createTokenAccount(context, buyToken1Mint, offerAuthority, BigInt(0), true);
        const offerStartTime = Date.now();
        const offerEndTime = offerStartTime + 7200;
        const bossBuyTokenAccount1 = createTokenAccount(context, buyToken1Mint, boss, BigInt(1_000e9));
        
        // when
        await expect(
            program.methods
                .makeOfferOne(offerId, new anchor.BN(500e9), 
                    new anchor.BN(200e9), new anchor.BN(400e9), 
                    new anchor.BN(offerStartTime), new anchor.BN(offerEndTime), new anchor.BN(7201))
                .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
                .rpc()
        // then
        ).rejects.toThrow();
    });

    test("Make an offer with same sell_token_start_amount and sell_token_end_amount", async () => {        
        // given
        const offerId = new anchor.BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerSellTokenPda = createTokenAccount(context, sellTokenMint, offerAuthority, BigInt(0), true);
        const offerBuyTokenPda = createTokenAccount(context, buyToken1Mint, offerAuthority, BigInt(0), true);
        const offerStartTime = Date.now();
        const offerEndTime = offerStartTime + 7200;
        const bossBuyTokenAccount1 = createTokenAccount(context, buyToken1Mint, boss, BigInt(1_000e9));

        // when
        await program.methods
            .makeOfferOne(offerId, new anchor.BN(500e9), 
                new anchor.BN(200e9), new anchor.BN(200e9), 
                new anchor.BN(Date.now()), new anchor.BN(Date.now() + 7200), new anchor.BN(3600))
            .accounts({ sellTokenMint, buyToken1Mint, state: statePda })
            .rpc();

        // then
        const offerAccount = await program.account.offer.fetch(offerPda);
        expect(offerAccount.sellTokenStartAmount.eq(new anchor.BN(200e9))).toBe(true);
        expect(offerAccount.sellTokenEndAmount.eq(new anchor.BN(200e9))).toBe(true);
    })
});