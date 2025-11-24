import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, Program } from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey, Transaction, TransactionInstruction } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, createAssociatedTokenAccountIdempotentInstruction } from "@solana/spl-token";
import { BN } from "bn.js";
import { Onreapp } from "../../target/types/onreapp";
import idl from "../../target/idl/onreapp.json";
import bs58 from "bs58";

// Environment configuration
// export const RPC_URL = process.env.SOL_MAINNET_RPC_URL || "https://api.mainnet-beta.solana.com";
export const RPC_URL = "https://api.devnet.solana.com";

// BOSS wallet addresses (Squad multisig accounts)
// export const BOSS = new PublicKey("45YnzauhsBM8CpUz96Djf8UG5vqq2Dua62wuW9H3jaJ5"); // WARN: SQUAD MAIN ACCOUNT!!!
// export const BOSS = new PublicKey("7rzEKejyAXJXMkGfRhMV9Vg1k7tFznBBEFu3sfLNz8LC"); // DEV Squad
export const BOSS = new PublicKey("EVdiVScB7LX1P3bn7ZLmLJTBrSSgRXPqRU3bVxrEpRb5"); // devnet Squad
// Note: In production, the actual boss is fetched from the program state, these are just for reference

// Default token mints - UPDATE THESE for your environment
export const TOKEN_IN_MINT = new PublicKey("2eW3HJzbgrCnV1fd7dUbyPj5T95D35oBPcJyfXtoGNrw"); // USDC-like (6 decimals)
export const TOKEN_OUT_MINT = new PublicKey("6WLYBF2o3RSkZ9SoNhhFYxUPYzLaa83xSTZ3o46cg4CN"); // ONyc-like (9 decimals)

/**
 * Helper class for Onre scripts - provides clean abstraction similar to test OnreProgram
 * Encapsulates common functionality to reduce duplication across scripts
 */
export class ScriptHelper {
    program: Program<Onreapp>;
    connection: Connection;
    statePda: PublicKey;

    pdas: {
        offerVaultAuthorityPda: PublicKey;
        permissionlessVaultAuthorityPda: PublicKey;
        mintAuthorityPda: PublicKey;
    };

    private constructor(program: Program<Onreapp>, connection: Connection) {
        this.program = program;
        this.connection = connection;
        [this.statePda] = PublicKey.findProgramAddressSync([Buffer.from("state")], program.programId);

        this.pdas = {
            offerVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("offer_vault_authority")], program.programId)[0],
            permissionlessVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], program.programId)[0],
            mintAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("mint_authority")], program.programId)[0]
        };
    }

    /**
     * Create a new ScriptHelper instance with a dummy wallet
     */
    static async create(): Promise<ScriptHelper> {
        const connection = new Connection(RPC_URL);
        const wallet = new anchor.Wallet(Keypair.generate());
        const provider = new AnchorProvider(connection, wallet);
        const program = new Program<Onreapp>(idl as Onreapp, provider);

        anchor.setProvider(provider);

        return new ScriptHelper(program, connection);
    }

    /**
     * Create a ScriptHelper with a specific wallet/signer
     */
    static async createWithWallet(wallet: anchor.Wallet): Promise<ScriptHelper> {
        const connection = new Connection(RPC_URL);
        const provider = new AnchorProvider(connection, wallet);
        const program = new Program<Onreapp>(idl as Onreapp, provider);

        anchor.setProvider(provider);

        return new ScriptHelper(program, connection);
    }

    // Account getters
    async getBoss(): Promise<PublicKey> {
        const stateAccount = await this.program.account.state.fetch(this.statePda);
        return stateAccount.boss;
    }

    async getOffer(tokenInMint: PublicKey, tokenOutMint: PublicKey) {
        const offerPda = PublicKey.findProgramAddressSync(
            [Buffer.from("offer"), tokenInMint.toBuffer(), tokenOutMint.toBuffer()],
            this.program.programId
        )[0];
        return await this.program.account.offer.fetch(offerPda);
    }

    async getState() {
        return await this.program.account.state.fetch(this.statePda);
    }

    /**
     * Create instructions for permissionless token accounts if they don't exist
     * Returns an array of instructions (may be empty if accounts already exist)
     */
    async buildCreatePermissionlessTokenAccountsIxs(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        payer?: PublicKey | null;
    }): Promise<TransactionInstruction[]> {
        const instructions: TransactionInstruction[] = [];
        const permissionlessAuthority = this.pdas.permissionlessVaultAuthorityPda;
        const payer = params.payer ?? BOSS;

        // Create permissionless token_in account if it doesn't exist
        const permissionlessTokenInAccount = getAssociatedTokenAddressSync(
            params.tokenInMint,
            permissionlessAuthority,
            true,
            TOKEN_PROGRAM_ID
        );

        const tokenInAccountInfo = await this.connection.getAccountInfo(permissionlessTokenInAccount);
        if (!tokenInAccountInfo) {
            const createTokenInIx = createAssociatedTokenAccountIdempotentInstruction(
                payer,
                permissionlessTokenInAccount,
                permissionlessAuthority,
                params.tokenInMint,
                TOKEN_PROGRAM_ID
            );
            instructions.push(createTokenInIx);
        }

        // Create permissionless token_out account if it doesn't exist
        const permissionlessTokenOutAccount = getAssociatedTokenAddressSync(
            params.tokenOutMint,
            permissionlessAuthority,
            true,
            TOKEN_PROGRAM_ID
        );

        const tokenOutAccountInfo = await this.connection.getAccountInfo(permissionlessTokenOutAccount);
        if (!tokenOutAccountInfo) {
            const createTokenOutIx = createAssociatedTokenAccountIdempotentInstruction(
                payer,
                permissionlessTokenOutAccount,
                permissionlessAuthority,
                params.tokenOutMint,
                TOKEN_PROGRAM_ID
            );
            instructions.push(createTokenOutIx);
        }

        return instructions;
    }

    // Transaction builders - return unsigned transactions for signing
    async buildMakeOfferIx(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        feeBasisPoints?: number;
        needsApproval?: boolean;
        allowPermissionless?: boolean;
        tokenInProgram?: PublicKey;
        boss?: PublicKey;
    }) {
        const feeBasisPoints = params.feeBasisPoints ?? 0;
        const needsApproval = params.needsApproval ?? false;
        const allowPermissionless = params.allowPermissionless ?? false;

        return await this.program.methods
            .makeOffer(feeBasisPoints, needsApproval, allowPermissionless)
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutMint: params.tokenOutMint,
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildAddOfferVectorIx(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        baseTime: number;
        basePrice: number;
        apr: number;
        priceFixDuration: number;
        boss?: PublicKey;
    }) {
        return await this.program.methods
            .addOfferVector(
                new BN(params.baseTime),
                new BN(params.basePrice),
                new BN(params.apr),
                new BN(params.priceFixDuration)
            )
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildCloseOfferIx(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        boss?: PublicKey;
    }) {
        return await this.program.methods
            .closeOffer()
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildUpdateOfferFeeIx(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        newFeeBasisPoints: number;
        boss?: PublicKey;
    }) {
        return await this.program.methods
            .updateOfferFee(params.newFeeBasisPoints)
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildDeleteOfferVectorIx(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        vectorStartTimestamp: number;
        boss?: PublicKey;
    }) {
        return await this.program.methods
            .deleteOfferVector(new BN(params.vectorStartTimestamp))
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildTakeOfferIx(params: {
        tokenInAmount: number;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        approvalMessage?: any;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
        boss?: PublicKey;
    }) {
        return await this.program.methods
            .takeOffer(new BN(params.tokenInAmount), params.approvalMessage ?? null)
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildTakeOfferPermissionlessIx(params: {
        tokenInAmount: number;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        approvalMessage?: any;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
        boss?: PublicKey;
    }) {
        return await this.program.methods
            .takeOfferPermissionless(new BN(params.tokenInAmount), params.approvalMessage ?? null)
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildOfferVaultDepositIx(params: {
        amount: number,
        tokenMint: PublicKey,
        tokenProgram?: PublicKey,
        boss?: PublicKey;
    }) {
        return await this.program.methods
            .offerVaultDeposit(new BN(params.amount))
            .accountsPartial({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildOfferVaultWithdrawIx(params: {
        amount: number,
        tokenMint: PublicKey,
        tokenProgram?: PublicKey,
        boss?: PublicKey;
    }) {
        return await this.program.methods
            .offerVaultWithdraw(new BN(params.amount))
            .accountsPartial({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildAddAdminIx(params: { admin: PublicKey; boss?: PublicKey }) {
        return await this.program.methods
            .addAdmin(params.admin)
            .accountsPartial({
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildRemoveAdminIx(params: { admin: PublicKey; boss?: PublicKey }) {
        return await this.program.methods
            .removeAdmin(params.admin)
            .accountsPartial({
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildSetBossIx(params: { newBoss: PublicKey; boss?: PublicKey }) {
        return await this.program.methods
            .setBoss(params.newBoss)
            .accountsPartial({
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildSetKillSwitchIx(params: { enable: boolean; boss?: PublicKey }) {
        return await this.program.methods
            .setKillSwitch(params.enable)
            .accountsPartial({
                signer: params.boss ?? BOSS
            })
            .instruction();
    }

    async buildInitializeIx(params?: { payer?: PublicKey }) {
        return await this.program.methods
            .initialize()
            .accountsPartial({
                boss: params?.payer ?? BOSS
            })
            .instruction();
    }

    async buildMigrateStateIx(params?: { boss?: PublicKey }) {
        return await this.program.methods
            .migrateV3()
            .accountsPartial({
                state: this.statePda,
                permissionlessAuthority: this.pdas.permissionlessVaultAuthorityPda,
                boss: params?.boss ?? BOSS
            })
            .instruction();
    }

    async buildInitializeVaultAuthorityIx(params?: { boss?: PublicKey }) {
        return await this.program.methods
            .initializeVaultAuthority()
            .accountsPartial({
                boss: params?.boss ?? BOSS
            })
            .instruction();
    }

    async buildInitializeMintAuthorityIx(params?: { boss?: PublicKey }) {
        return await this.program.methods
            .initializeMintAuthority()
            .accountsPartial({
                boss: params?.boss ?? BOSS
            })
            .instruction();
    }

    async buildInitializePermissionlessAuthorityIx(params: { name: string; boss?: PublicKey }) {
        return await this.program.methods
            .initializePermissionlessAuthority(params.name)
            .accountsPartial({
                boss: params.boss ?? BOSS
            })
            .instruction();
    }

    async prepareTransactionMultipleIxs(ixs: TransactionInstruction[], boss?: PublicKey) {
        const tx = new Transaction();
        for (const ix of ixs) {
            tx.add(ix);
        }
        tx.feePayer = boss ?? BOSS;
        tx.recentBlockhash = (await this.connection.getLatestBlockhash()).blockhash;
        return tx;
    }

    // Helper to prepare transaction with boss as fee payer and recent blockhash
    async prepareTransaction(ix: TransactionInstruction, boss?: PublicKey) {
        return await this.prepareTransactionMultipleIxs([ix], boss);
    }

    /**
     * Serialize transaction to base58 for external signing
     */
    serializeTransaction(tx: Transaction): string {
        const serializedTx = tx.serialize({
            requireAllSignatures: false,
            verifySignatures: false
        });
        return bs58.encode(serializedTx);
    }

    /**
     * Utility to print transaction as base58 for external signing
     */
    printTransaction(tx: Transaction, title: string = "Transaction") {
        const base58Tx = this.serializeTransaction(tx);
        console.log(`${title} (Base58):`);
        console.log(base58Tx);
        return base58Tx;
    }
}
