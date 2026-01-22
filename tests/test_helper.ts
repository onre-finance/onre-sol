import { Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import {
    ACCOUNT_SIZE,
    AccountLayout,
    createAssociatedTokenAccountInstruction,
    createInitializeMint2Instruction,
    createInitializeTransferFeeConfigInstruction,
    createMintToInstruction,
    ExtensionType,
    getAssociatedTokenAddressSync,
    getMintLen,
    MINT_SIZE,
    MintLayout,
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import { ComputeBudget, FeatureSet, LiteSVM } from "litesvm";
import idl from "../target/idl/onreapp.json";

export const ONREAPP_PROGRAM_ID = new PublicKey((idl as any).address);
export const INITIAL_LAMPORTS = 1_000_000_000; // 1 SOL
export const BPF_UPGRADEABLE_LOADER_PROGRAM_ID = new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111");

// LiteSVM transaction results are objects with method accessors like:
// - result.err() - returns error object
// - result.meta() - returns metadata object
// - meta.logs() - returns array of log strings
// - meta.computeUnitsConsumed() - returns number
// - result.toString() - returns formatted error string
// These are NOT properties, they are methods that must be called

export class TestHelper {
    svm: LiteSVM;
    payer: Keypair;

    private constructor(svm: LiteSVM, payer: Keypair) {
        this.svm = svm;
        this.payer = payer;
    }

    static async create() {
        const svm = new LiteSVM().withFeatureSet(FeatureSet.allEnabled()).withPrecompiles();
        const payer = Keypair.generate();

        // Set initial clock to a non-zero timestamp (e.g., Jan 1, 2024)
        const clock = svm.getClock();
        clock.unixTimestamp = BigInt(1704067200); // Jan 1, 2024 00:00:00 UTC
        svm.setClock(clock);

        svm.airdrop(payer.publicKey, BigInt(100_000_000_000)); // 100 SOL

        // Load and deploy the program as upgradeable
        const fs = await import("fs");
        const path = await import("path");
        const programPath = path.join(process.cwd(), "target/deploy/onreapp.so");
        const programBytes = fs.readFileSync(programPath);

        // Create programData PDA
        const programDataPda = PublicKey.findProgramAddressSync(
            [ONREAPP_PROGRAM_ID.toBuffer()],
            BPF_UPGRADEABLE_LOADER_PROGRAM_ID
        )[0];

        // Set up the programData account FIRST (contains actual bytecode)
        const programDataAccountData = Buffer.alloc(45 + programBytes.length);
        programDataAccountData.writeUInt32LE(3, 0); // UpgradeableLoaderState::ProgramData discriminator
        programDataAccountData.writeBigUInt64LE(BigInt(0), 4); // slot (u64)
        programDataAccountData.writeUInt8(1, 12); // Option::Some for upgrade_authority
        payer.publicKey.toBuffer().copy(programDataAccountData, 13); // upgrade_authority_address (32 bytes)
        programBytes.copy(programDataAccountData, 45); // actual program bytecode

        svm.setAccount(programDataPda, {
            executable: false,
            data: programDataAccountData,
            lamports: 10_000_000,
            owner: BPF_UPGRADEABLE_LOADER_PROGRAM_ID
        });

        // Set up the program account (executable, points to programData)
        const programAccountData = Buffer.alloc(36);
        programAccountData.writeUInt32LE(2, 0); // UpgradeableLoaderState::Program discriminator
        programDataPda.toBuffer().copy(programAccountData, 4); // programdata_address (32 bytes)

        svm.setAccount(ONREAPP_PROGRAM_ID, {
            executable: true,
            data: programAccountData,
            lamports: 1_000_000,
            owner: BPF_UPGRADEABLE_LOADER_PROGRAM_ID
        });

        return new TestHelper(svm, payer);
    }


    async advanceSlot() {
        // Advance the slot and expire the blockhash to force new transactions
        const clock = this.svm.getClock();
        this.svm.warpToSlot(clock.slot + BigInt(1));
        this.svm.expireBlockhash();
    }

    getBoss(): PublicKey {
        return this.payer.publicKey;
    }

    createUserAccount(): Keypair {
        const user = Keypair.generate();
        this.svm.airdrop(user.publicKey, BigInt(INITIAL_LAMPORTS));
        return user;
    }

    createMint2022(decimals: number, mintAuthority: PublicKey | null = null, freezeAuthority: PublicKey | null = mintAuthority): PublicKey {
        return this.createMint(decimals, mintAuthority, BigInt(999_999_999) * (10n ** BigInt(decimals)), freezeAuthority, TOKEN_2022_PROGRAM_ID);
    }

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

        const extensions = [ExtensionType.TransferFeeConfig];
        const mintLen = getMintLen(extensions);

        const createAccountIx = SystemProgram.createAccount({
            fromPubkey: this.payer.publicKey,
            newAccountPubkey: mint.publicKey,
            space: mintLen,
            lamports: INITIAL_LAMPORTS,
            programId: TOKEN_2022_PROGRAM_ID
        });

        const initTransferFeeIx = createInitializeTransferFeeConfigInstruction(
            mint.publicKey,
            mintAuth,
            mintAuth,
            transferFeeBasisPoints,
            maxFee,
            TOKEN_2022_PROGRAM_ID
        );

        const initMintIx = createInitializeMint2Instruction(
            mint.publicKey,
            decimals,
            mintAuth,
            freezeAuth,
            TOKEN_2022_PROGRAM_ID
        );

        const tx = new Transaction().add(createAccountIx, initTransferFeeIx, initMintIx);

        const isMintAuthorityPda = mintAuthority !== null && mintAuthority.toBase58() !== this.getBoss().toBase58();

        if (!isMintAuthorityPda) {
            const bossAta = getAssociatedTokenAddressSync(
                mint.publicKey,
                this.getBoss(),
                false,
                TOKEN_2022_PROGRAM_ID
            );

            const createAtaIx = createAssociatedTokenAccountInstruction(
                this.payer.publicKey,
                bossAta,
                this.getBoss(),
                mint.publicKey,
                TOKEN_2022_PROGRAM_ID
            );

            const initialSupply = BigInt(999_999_999) * (10n ** BigInt(decimals));
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

        await this.sendAndConfirmTransaction(tx, [this.payer, mint]);

        return mint.publicKey;
    }

    createMint(
        decimals: number,
        mintAuthority: PublicKey | null = null,
        supply: bigint = BigInt(999_999_999) * (10n ** BigInt(decimals)),
        freezeAuthority: PublicKey | null = mintAuthority,
        owner: PublicKey = TOKEN_PROGRAM_ID
    ): PublicKey {
        const mintData = Buffer.alloc(MINT_SIZE);
        MintLayout.encode({
            mintAuthorityOption: 1,
            mintAuthority: mintAuthority ? mintAuthority : this.getBoss(),
            supply: BigInt(supply),
            decimals: decimals,
            isInitialized: true,
            freezeAuthorityOption: 1,
            freezeAuthority: freezeAuthority ? freezeAuthority : this.getBoss()
        }, mintData);

        const mintAddress = PublicKey.unique();

        this.svm.setAccount(mintAddress, {
            executable: false,
            data: mintData,
            lamports: INITIAL_LAMPORTS,
            owner
        });

        return mintAddress;
    }

    createTokenAccount(
        mint: PublicKey,
        owner: PublicKey,
        amount: bigint,
        allowOwnerOffCurve: boolean = false,
        programId: PublicKey = TOKEN_PROGRAM_ID
    ): PublicKey {
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

        this.svm.setAccount(tokenAccountAddress, {
            executable: false,
            data: tokenAccountData,
            lamports: INITIAL_LAMPORTS,
            owner: programId
        });

        return tokenAccountAddress;
    }

    async createToken2022Account(mint: PublicKey, owner: PublicKey): Promise<PublicKey> {
        const tokenAccount = getAssociatedTokenAddressSync(
            mint,
            owner,
            true,
            TOKEN_2022_PROGRAM_ID
        );

        const createAtaIx = createAssociatedTokenAccountInstruction(
            this.payer.publicKey,
            tokenAccount,
            owner,
            mint,
            TOKEN_2022_PROGRAM_ID
        );

        const tx = new Transaction().add(createAtaIx);
        await this.sendAndConfirmTransaction(tx, [this.payer]);

        return tokenAccount;
    }

    async getTokenAccountBalance(tokenAccount: PublicKey): Promise<bigint> {
        const tokenAccountData = await this.getAccount(tokenAccount);
        return tokenAccountData.amount;
    }

    async getAccount(accountAddress: PublicKey) {
        const account = this.svm.getAccount(accountAddress);
        if (!account) {
            throw new Error("Token account not found");
        }

        return AccountLayout.decode(account.data);
    }

    async getMintInfo(mint: PublicKey): Promise<{ supply: bigint, decimals: number, mintAuthority: PublicKey | null }> {
        const account = this.svm.getAccount(mint);
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
        const account = this.svm.getAccount(tokenAccount);
        const tokenAccountData = AccountLayout.decode(account!.data);
        expect(tokenAccountData.amount).toBe(amount);
    }

    async getCurrentClockTime() {
        const clock = this.svm.getClock();
        return Number(clock.unixTimestamp);
    }

    async advanceClockBy(seconds: number) {
        const clock = this.svm.getClock();
        clock.unixTimestamp += BigInt(seconds);
        this.svm.setClock(clock);
    }

    async getAccountInfo(publicKey: PublicKey) {
        return this.svm.getAccount(publicKey);
    }

    async getTokenAccount(tokenAccount: PublicKey) {
        const account = this.svm.getAccount(tokenAccount);
        if (!account) {
            throw new Error("Token account not found");
        }
        return AccountLayout.decode(account.data);
    }

    // Helper to send transactions
    async sendAndConfirmTransaction(tx: Transaction, signers: Keypair[]) {
        tx.recentBlockhash = this.svm.latestBlockhash();
        tx.feePayer = this.payer.publicKey;
        tx.sign(...signers);

        const result = this.svm.sendTransaction(tx);

        if ("Err" in result) {
            throw new Error(`Transaction failed: ${JSON.stringify(result.Err)}`);
        }

        return result;
    }

    // Warp methods for compatibility
    warpToSlot(slot: bigint) {
        this.svm.warpToSlot(slot);
    }

    setClock(clock: any) {
        this.svm.setClock(clock);
    }

    setAccount(pubkey: PublicKey, account: {
        executable: boolean;
        data: Uint8Array | Buffer;
        lamports: number;
        owner: PublicKey
    }) {
        this.svm.setAccount(pubkey, account);
    }

    get context() {
        // For compatibility with OnreProgram
        return this;
    }

    get lastBlockhash() {
        return this.svm.latestBlockhash();
    }

    // Create a connection-like object for Anchor provider
    getConnection() {
        const svm = this.svm;
        const payer = this.payer;
        return {
            getLatestBlockhash: async () => ({
                blockhash: svm.latestBlockhash(),
                lastValidBlockHeight: 0
            }),
            getMinimumBalanceForRentExemption: async () => 890880,
            getAccountInfo: async (pubkey: PublicKey) => {
                const account = svm.getAccount(pubkey);
                if (!account) return null;
                return { ...account, data: Buffer.from(account.data) };
            },
            getAccountInfoAndContext: async (pubkey: PublicKey) => {
                const account = svm.getAccount(pubkey);
                const value = account ? { ...account, data: Buffer.from(account.data) } : null;
                return { context: { slot: 0 }, value };
            },
            sendRawTransaction: async (rawTransaction: Buffer | Uint8Array) => {
                const tx = Transaction.from(rawTransaction);
                const budget = new ComputeBudget();
                budget.computeUnitLimit = BigInt(1_400_000);
                svm.withComputeBudget(budget);
                const result = svm.sendTransaction(tx);

                // Check if it's a FailedTransactionMetadata (has err() method)
                if (typeof result.err === "function") {
                    const logs = result.meta().logs();
                    // Use result.toString() which includes the full error info
                    const error: any = new Error(result.toString());
                    error.logs = logs;
                    throw error;
                }

                return "signature";
            },
            confirmTransaction: async () => ({ value: { err: null } }),
            _rpcRequest: async (method: string, args: any[]) => {
                if (method === "simulateTransaction") {
                    const tx = Transaction.from(Buffer.from(args[0], "base64"));

                    // Sign if not signed
                    if (!tx.signatures.some(sig => sig.signature !== null)) {
                        tx.recentBlockhash = svm.latestBlockhash();
                        tx.feePayer = payer.publicKey;
                        tx.partialSign(payer);
                    }

                    const result = svm.simulateTransaction(tx);

                    if ("Err" in result) {
                        const err = result.Err;
                        const meta = err.meta();
                        // Return error in Solana RPC format
                        return {
                            context: { slot: 0 },
                            value: {
                                err: err.err(),
                                logs: meta.logs(),
                                accounts: null,
                                unitsConsumed: Number(meta.computeUnitsConsumed()),
                                returnData: null
                            }
                        };
                    }

                    const meta = result.meta();
                    const returnData = meta.returnData();

                    return {
                        context: { slot: 0 },
                        value: {
                            err: null,
                            logs: meta.logs(),
                            accounts: null,
                            unitsConsumed: Number(meta.computeUnitsConsumed()),
                            returnData: returnData && returnData.data().length > 0 ? {
                                programId: Buffer.from(returnData.programId()).toString("base64"),
                                data: [Buffer.from(returnData.data()).toString("base64"), "base64"]
                            } : null
                        }
                    };
                }
                throw new Error(`Unsupported RPC method: ${method}`);
            }
        };
    }
}
