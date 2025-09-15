import { AddedProgram, Clock, ProgramTestContext, startAnchor } from "solana-bankrun";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import {
    ACCOUNT_SIZE,
    AccountLayout,
    getAssociatedTokenAddressSync,
    MINT_SIZE,
    MintLayout,
    TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import idl from "../target/idl/onreapp.json";

export const ONREAPP_PROGRAM_ID = new PublicKey((idl as any).address);
export const INITIAL_LAMPORTS = 1_000_000_000; // 1 SOL

export class TestHelper {
    context: ProgramTestContext;

    private constructor(context: ProgramTestContext) {
        this.context = context;
    }

    static async create() {
        const programInfo: AddedProgram = {
            programId: ONREAPP_PROGRAM_ID,
            name: "onreapp"
        };
        const context = await startAnchor(process.cwd(), [programInfo], []);

        return new TestHelper(context);
    }

    getBoss(): PublicKey {
        return this.context.payer.publicKey;
    }

    // Account helper functions
    createUserAccount(): Keypair {
        const user = Keypair.generate();
        this.context.setAccount(user.publicKey, {
            executable: false,
            data: new Uint8Array([]),
            lamports: INITIAL_LAMPORTS,
            owner: SystemProgram.programId
        });

        return user;
    }

    createMint(decimals: number, mintAuthority: PublicKey = null, freezeAuthority: PublicKey = mintAuthority): PublicKey {
        if (!mintAuthority) {
            mintAuthority = this.getBoss();
            freezeAuthority = this.getBoss();
        }
        const mintData = Buffer.alloc(MINT_SIZE);
        MintLayout.encode({
            mintAuthorityOption: 1,  // 1 = Some(authority), 0 = None
            mintAuthority: mintAuthority,
            supply: BigInt(0),
            decimals: decimals,
            isInitialized: true,
            freezeAuthorityOption: 1,  // 1 = Some(authority), 0 = None
            freezeAuthority: freezeAuthority
        }, mintData);

        const mintAddress = PublicKey.unique();
        this.context.setAccount(mintAddress, {
            executable: false,
            data: mintData,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_PROGRAM_ID
        });

        return mintAddress;
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
            closeAuthority: PublicKey.default
        }, tokenAccountData);

        const tokenAccountAddress = getAssociatedTokenAddressSync(mint, owner, allowOwnerOffCurve);

        this.context.setAccount(tokenAccountAddress, {
            executable: false,
            data: tokenAccountData,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_PROGRAM_ID
        });

        return tokenAccountAddress;
    }

    getAssociatedTokenAccount(mint: PublicKey, owner: PublicKey): PublicKey {
        return getAssociatedTokenAddressSync(mint, owner);
    }

    async getTokenAccountBalance(tokenAccount: PublicKey): Promise<bigint> {
        const tokenAccountData = await this.getAccount(tokenAccount);
        return tokenAccountData.amount;
    }

    async getAccount(accountAddress: PublicKey) {
        const account = await this.context.banksClient.getAccount(accountAddress);
        if (!account) {
            throw new Error("Token account not found");
        }

        return AccountLayout.decode(account.data); // TODO: Check if works with mint accounts as well
    }

    async getMintInfo(mint: PublicKey): Promise<{ supply: bigint, decimals: number, mintAuthority: PublicKey | null }> {
        const account = await this.context.banksClient.getAccount(mint);
        if (!account) {
            throw new Error("Mint account not found");
        }
        const mintData = MintLayout.decode(account.data);
        return {
            supply: mintData.supply,
            decimals: mintData.decimals,
            mintAuthority: mintData.mintAuthorityOption === 1 ? mintData.mintAuthority : null
        };
    }

    async expectTokenAccountAmountToBe(tokenAccount: PublicKey, amount: bigint) {
        const account = await this.context.banksClient.getAccount(tokenAccount);
        const tokenAccountData = AccountLayout.decode(account!.data);
        expect(tokenAccountData.amount).toBe(amount);
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
                clock.unixTimestamp + BigInt(seconds)
            )
        );
    }
}

