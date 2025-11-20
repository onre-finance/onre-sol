import { AddedProgram, Clock, ProgramTestContext, startAnchor } from "solana-bankrun";
import { Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import {
    ACCOUNT_SIZE,
    AccountLayout,
    getAssociatedTokenAddressSync,
    MINT_SIZE,
    MintLayout,
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    ExtensionType,
    getMintLen,
    createInitializeMint2Instruction,
    createInitializeTransferFeeConfigInstruction,
    createAssociatedTokenAccountInstruction,
    createMintToInstruction,
    getAccountLen,
    createInitializeAccountInstruction,
    createTransferCheckedWithFeeInstruction,
    getMint,
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

    async advanceSlot() {
        const currentSlot = await this.context.banksClient.getSlot();
        this.context.warpToSlot(currentSlot + BigInt(1));
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

    createMint2022(decimals: number, mintAuthority: PublicKey | null = null, freezeAuthority: PublicKey | null = mintAuthority): PublicKey {
        return this.createMint(decimals, mintAuthority, BigInt(999_999_999 * 10 ** decimals), freezeAuthority, TOKEN_2022_PROGRAM_ID);
    }

    /**
     * Creates a Token-2022 mint with transfer fee extension using proper SPL Token instructions
     * @param decimals Number of decimals for the token
     * @param transferFeeBasisPoints Transfer fee in basis points (e.g., 100 = 1%)
     * @param maxFee Maximum fee per transfer
     * @param mintAuthority Optional mint authority (defaults to boss)
     * @param freezeAuthority Optional freeze authority (defaults to mintAuthority)
     * @returns PublicKey of the created mint
     */
    async createMint2022WithTransferFee(
        decimals: number,
        transferFeeBasisPoints: number,
        maxFee: bigint,
        mintAuthority: PublicKey | null = null,
        freezeAuthority: PublicKey | null = mintAuthority
    ): Promise<PublicKey> {
        const mint = Keypair.generate();
        const mintAuth = mintAuthority || this.getBoss();
        const freezeAuth = freezeAuthority || mintAuth;

        // Calculate the space required for the mint with TransferFeeConfig extension
        const extensions = [ExtensionType.TransferFeeConfig];
        const mintLen = getMintLen(extensions);

        // Create account for the mint
        const createAccountIx = SystemProgram.createAccount({
            fromPubkey: this.context.payer.publicKey,
            newAccountPubkey: mint.publicKey,
            space: mintLen,
            lamports: INITIAL_LAMPORTS,
            programId: TOKEN_2022_PROGRAM_ID,
        });

        // Initialize the transfer fee config extension
        const initTransferFeeIx = createInitializeTransferFeeConfigInstruction(
            mint.publicKey,
            mintAuth,
            mintAuth,
            transferFeeBasisPoints,
            maxFee,
            TOKEN_2022_PROGRAM_ID
        );

        // Initialize the mint
        const initMintIx = createInitializeMint2Instruction(
            mint.publicKey,
            decimals,
            mintAuth,
            freezeAuth,
            TOKEN_2022_PROGRAM_ID
        );

        // Create transaction with mint initialization
        const tx = new Transaction().add(
            createAccountIx,
            initTransferFeeIx,
            initMintIx
        );

        // Only mint initial supply if mint authority is not a PDA
        // PDAs can't sign transactions, only CPIs
        const isMintAuthorityPda = mintAuthority !== null && mintAuthority.toBase58() !== this.getBoss().toBase58();

        if (!isMintAuthorityPda) {
            // Create ATA for boss to receive initial supply
            const bossAta = getAssociatedTokenAddressSync(
                mint.publicKey,
                this.getBoss(),
                false,
                TOKEN_2022_PROGRAM_ID
            );

            const createAtaIx = createAssociatedTokenAccountInstruction(
                this.context.payer.publicKey,
                bossAta,
                this.getBoss(),
                mint.publicKey,
                TOKEN_2022_PROGRAM_ID
            );

            // Mint initial supply to boss (999,999,999 tokens)
            const initialSupply = BigInt(999_999_999 * 10 ** decimals);
            const mintToIx = createMintToInstruction(
                mint.publicKey,
                bossAta,
                mintAuth,
                initialSupply,
                [],
                TOKEN_2022_PROGRAM_ID
            );

            tx.add(createAtaIx, mintToIx);
        }

        // Sign and process transaction
        tx.recentBlockhash = this.context.lastBlockhash;
        tx.sign(this.context.payer, mint);

        await this.context.banksClient.processTransaction(tx);

        return mint.publicKey;
    }

    createMint(decimals: number, mintAuthority: PublicKey | null = null, supply: bigint = BigInt(999_999_999 * 10 ** decimals), freezeAuthority: PublicKey | null = mintAuthority, owner: PublicKey = TOKEN_PROGRAM_ID): PublicKey {
        const mintData = Buffer.alloc(MINT_SIZE);
        MintLayout.encode({
            mintAuthorityOption: 1,  // 1 = Some(authority), 0 = None
            mintAuthority: mintAuthority ? mintAuthority : this.getBoss(),
            supply: BigInt(supply),
            decimals: decimals,
            isInitialized: true,
            freezeAuthorityOption: 1,  // 1 = Some(authority), 0 = None
            freezeAuthority: freezeAuthority ? freezeAuthority : this.getBoss()
        }, mintData);

        const mintAddress = PublicKey.unique();
        this.context.setAccount(mintAddress, {
            executable: false,
            data: mintData,
            lamports: INITIAL_LAMPORTS,
            owner
        });

        return mintAddress;
    };

    createTokenAccount(mint: PublicKey, owner: PublicKey, amount: bigint, allowOwnerOffCurve: boolean = false, programId: PublicKey = TOKEN_PROGRAM_ID): PublicKey {
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

        const tokenAccountAddress = getAssociatedTokenAddressSync(mint, owner, allowOwnerOffCurve, programId);

        this.context.setAccount(tokenAccountAddress, {
            executable: false,
            data: tokenAccountData,
            lamports: INITIAL_LAMPORTS,
            owner: programId
        });

        return tokenAccountAddress;
    }

    /**
     * Creates a Token-2022 token account using proper SPL Token instructions
     * This is needed for Token-2022 mints with extensions like TransferFee
     * For PDAs, creates a regular account; for regular owners, creates an ATA
     */
    async createToken2022Account(mint: PublicKey, owner: PublicKey): Promise<PublicKey> {
            // For regular owners, use ATA
            const tokenAccount = getAssociatedTokenAddressSync(
                mint,
                owner,
                true,
                TOKEN_2022_PROGRAM_ID
            );

            const createAtaIx = createAssociatedTokenAccountInstruction(
                this.context.payer.publicKey,
                tokenAccount,
                owner,
                mint,
                TOKEN_2022_PROGRAM_ID
            );

            const tx = new Transaction().add(createAtaIx);
            tx.recentBlockhash = this.context.lastBlockhash;
            tx.sign(this.context.payer);

            await this.context.banksClient.processTransaction(tx);

            return tokenAccount;
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

    async getAccountInfo(publicKey: PublicKey) {
        return await this.context.banksClient.getAccount(publicKey);
    }

    async getTokenAccount(tokenAccount: PublicKey) {
        const account = await this.context.banksClient.getAccount(tokenAccount);
        if (!account) {
            throw new Error("Token account not found");
        }
        return AccountLayout.decode(account.data);
    }
}

