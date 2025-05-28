import { Clock, ProgramTestContext } from "solana-bankrun"
import { OnreApp } from "../target/types/onre_app"
import { BN, Program } from "@coral-xyz/anchor"
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { ACCOUNT_SIZE, AccountLayout, getAssociatedTokenAddressSync, MINT_SIZE, MintLayout, TOKEN_PROGRAM_ID } from "@solana/spl-token";

export const ONREAPP_PROGRAM_ID = new PublicKey("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe");
export const INITIAL_LAMPORTS = 1_000_000_000; // 1 SOL

export class TestHelper {
    context: ProgramTestContext;
    program: Program<OnreApp>;

    // accounts
    statePda: PublicKey;

    constructor(context: ProgramTestContext, program: Program<OnreApp>) {
        this.context = context;
        this.program = program;
        [this.statePda] = PublicKey.findProgramAddressSync([Buffer.from('state')], ONREAPP_PROGRAM_ID);
    }
    
    createUserAccount(): Keypair {
        const user = Keypair.generate();
        this.context.setAccount(user.publicKey, {
            executable: false,
            data: new Uint8Array([]),
            lamports: INITIAL_LAMPORTS,
            owner: SystemProgram.programId,
        });
    
        return user;
    }

    createMint(mintAuthority: PublicKey, supply: bigint = BigInt(100_000e9), decimals: number = 9, freezeAuthority: PublicKey = mintAuthority): PublicKey {
        const mintData = Buffer.alloc(MINT_SIZE);
        MintLayout.encode({
            mintAuthorityOption: 0,
            mintAuthority: mintAuthority,
            supply: supply,
            decimals: decimals,
            isInitialized: true,
            freezeAuthorityOption: 0,
            freezeAuthority: freezeAuthority,
        }, mintData)
        
        const mintAddress = PublicKey.unique();
        this.context.setAccount(mintAddress, {
            executable: false,
            data: mintData,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_PROGRAM_ID,
        });
    
        return mintAddress
    };

    createTokenAccount(mint: PublicKey, owner: PublicKey, amount: bigint, allowOwnerOffCurve: boolean = false): PublicKey {
        const tokenAccountData = Buffer.alloc(ACCOUNT_SIZE);
        AccountLayout.encode({
            mint: mint,
            owner: owner,
            amount: amount,
            delegateOption: 0,
            delegate: PublicKey.default,
            state: 1,
            isNativeOption: 0,
            isNative: BigInt(0),
            delegatedAmount: BigInt(0),
            closeAuthorityOption: 0,
            closeAuthority: PublicKey.default,
        }, tokenAccountData);
    
        const tokenAccountAddress = getAssociatedTokenAddressSync(mint, owner, allowOwnerOffCurve);
    
        this.context.setAccount(tokenAccountAddress, {
            executable: false,
            data: tokenAccountData,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_PROGRAM_ID,
        });
    
        return tokenAccountAddress;
    }

    createOneTokenOfferAccounts( 
        sellTokenMint: PublicKey, 
        offerSellTokenAmount: bigint = BigInt(0),
        buyTokenMint: PublicKey,
        offerBuyTokenAmount: bigint = BigInt(0),
        boss: PublicKey,
        bossBuyTokenAmount: bigint = BigInt(0),
    ): OfferOneTokenAccounts {
        const offerId = new BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerSellTokenPda = this.createTokenAccount(sellTokenMint, offerAuthority, offerSellTokenAmount, true);
        const offerBuyTokenPda = this.createTokenAccount(buyTokenMint, offerAuthority, offerBuyTokenAmount, true);
        const bossBuyTokenAccount = this.createTokenAccount(buyTokenMint, boss, bossBuyTokenAmount);
    
        return {
            offerId,
            offerAuthority,
            offerPda,
            offerSellTokenPda,
            offerBuyTokenPda,
            bossBuyTokenAccount,
        }
    }

    createTwoTokenOfferAccounts( 
        sellTokenMint: PublicKey, 
        offerSellTokenAmount: bigint = BigInt(0),
        buyToken1Mint: PublicKey,
        offerBuyToken1Amount: bigint = BigInt(0),
        buyToken2Mint: PublicKey,
        offerBuyToken2Amount: bigint = BigInt(0),
        boss: PublicKey,
        bossBuyTokenAmount1: bigint = BigInt(0),
        bossBuyTokenAmount2: bigint = BigInt(0),
    ): OfferTwoTokenAccounts {
        const offerId = new BN(PublicKey.unique().toBytes());
        const [offerAuthority] = PublicKey.findProgramAddressSync([Buffer.from('offer_authority'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const [offerPda] = PublicKey.findProgramAddressSync([Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)], ONREAPP_PROGRAM_ID);
        const offerSellTokenPda = this.createTokenAccount(sellTokenMint, offerAuthority, offerSellTokenAmount, true);
        const offerBuyToken1Pda = this.createTokenAccount(buyToken1Mint, offerAuthority, offerBuyToken1Amount, true);
        const offerBuyToken2Pda = this.createTokenAccount(buyToken2Mint, offerAuthority, offerBuyToken2Amount, true);
        const bossBuyTokenAccount1 = this.createTokenAccount(buyToken1Mint, boss, bossBuyTokenAmount1);
        const bossBuyTokenAccount2 = this.createTokenAccount(buyToken2Mint, boss, bossBuyTokenAmount2);
    
        return {
            offerId,
            offerAuthority,
            offerPda,
            offerSellTokenPda,
            offerBuyToken1Pda,
            offerBuyToken2Pda,
            bossBuyTokenAccount1,
            bossBuyTokenAccount2,
        }
    }

    async expectTokenAccountAmountToBe(tokenAccount: PublicKey, amount: bigint) {
        const account = await this.context.banksClient.getAccount(tokenAccount);
        const tokenAccountData = AccountLayout.decode(account!.data);
        expect(tokenAccountData.amount).toBe(amount);
    }

    async makeOfferOne(
        params: MakeOfferOneParams
    ) {
        return await this.program.methods
            .makeOfferOne(
                params.offerId, 
                new BN(params.buyTokenTotalAmount), 
                new BN(params.sellTokenStartAmount), 
                new BN(params.sellTokenEndAmount), 
                new BN(params.offerStartTime), 
                new BN(params.offerEndTime), 
                new BN(params.priceFixDuration))
            .accounts({
                sellTokenMint: params.sellTokenMint,
                buyToken1Mint: params.buyTokenMint,
                state: this.statePda,
            })
            .rpc();
    }

    async makeOfferTwo(params: MakeOfferTwoParams) {
        return await this.program.methods
            .makeOfferTwo(
                params.offerId, 
                new BN(params.buyToken1TotalAmount), 
                new BN(params.buyToken2TotalAmount), 
                new BN(params.sellTokenStartAmount), 
                new BN(params.sellTokenEndAmount), 
                new BN(params.offerStartTime), 
                new BN(params.offerEndTime), 
                new BN(params.priceFixDuration))
            .accounts({
                sellTokenMint: params.sellTokenMint,
                buyToken1Mint: params.buyToken1Mint,
                buyToken2Mint: params.buyToken2Mint,
                state: this.statePda,
            })
            .rpc();
    }

    async takeOfferOne(params: TakeOfferParams) {
        return await this.program.methods
            .takeOfferOne(
                new BN(params.sellTokenAmount))
            .accounts({ offer: params.offerPda, user: params.user.publicKey })
            .signers([params.user])
            .rpc();
    }

    async takeOfferTwo(params: TakeOfferParams) {
        return await this.program.methods
            .takeOfferTwo(
                new BN(params.sellTokenAmount))
            .accounts({ offer: params.offerPda, user: params.user.publicKey })
            .signers([params.user])
            .rpc();
    }

    async closeOfferOne(offerPda: PublicKey) {
        return await this.program.methods
            .closeOfferOne()
            .accounts({ offer: offerPda, state: this.statePda })
            .rpc();
    }

    async closeOfferTwo(offerPda: PublicKey) {
        return await this.program.methods
            .closeOfferTwo()
            .accounts({ offer: offerPda, state: this.statePda })
            .rpc();
    }

    async getOfferAccount(account: PublicKey) {
        return await this.program.account.offer.fetch(account);
    }

    async getCurrentClockTime() {
        const clock = await this.context.banksClient.getClock();
        return Number(clock.unixTimestamp);
    }

    async advanceClockBy(seconds: number) {
        const clock = await this.context.banksClient.getClock();
        this.context.setClock(
            new Clock(
                clock.slot,
                clock.epochStartTimestamp,
                clock.epoch,
                clock.leaderScheduleEpoch,
                clock.unixTimestamp + BigInt(seconds),
            )
        )
    }
}

type OfferOneTokenAccounts = {
    offerId: BN;
    offerAuthority: PublicKey;
    offerPda: PublicKey;
    offerSellTokenPda: PublicKey;
    offerBuyTokenPda: PublicKey;
    bossBuyTokenAccount: PublicKey;
}

type OfferTwoTokenAccounts = {
    offerId: BN;
    offerAuthority: PublicKey;
    offerPda: PublicKey;
    offerSellTokenPda: PublicKey;
    offerBuyToken1Pda: PublicKey;
    offerBuyToken2Pda: PublicKey;
    bossBuyTokenAccount1: PublicKey;
    bossBuyTokenAccount2: PublicKey;
}

type MakeOfferOneParams = {
    offerId: BN;
    buyTokenTotalAmount: number;
    sellTokenStartAmount: number;
    sellTokenEndAmount: number;
    offerStartTime: number;
    offerEndTime: number;
    priceFixDuration: number;
    sellTokenMint: PublicKey;
    buyTokenMint: PublicKey;
}

type MakeOfferTwoParams = {
    offerId: BN;
    buyToken1TotalAmount: number;
    buyToken2TotalAmount: number;
    sellTokenStartAmount: number;
    sellTokenEndAmount: number;
    offerStartTime: number;
    offerEndTime: number;
    priceFixDuration: number;
    sellTokenMint: PublicKey;
    buyToken1Mint: PublicKey;
    buyToken2Mint: PublicKey;
}

type TakeOfferParams = {
    sellTokenAmount: number;
    offerPda: PublicKey;
    user: Keypair;
}
