import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, Program } from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey, Transaction, TransactionInstruction } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { BN } from "bn.js";
import { Onreapp } from "../../target/types/onreapp";
import idl from "../../target/idl/onreapp.json";
import bs58 from "bs58";

// Environment configuration
// export const RPC_URL = process.env.SOL_MAINNET_RPC_URL || "https://api.mainnet-beta.solana.com";
export const RPC_URL = "https://api.devnet.solana.com";

// Program IDs
// export const PROGRAM_ID = new PublicKey("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe"); // PROD
export const PROGRAM_ID = new PublicKey("J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2"); // TEST + devnet
// export const PROGRAM_ID = new PublicKey("devHfQHgiFNifkLW49RCXpyTUZMyKuBNnFSbrQ8XsbX"); // DEV

// BOSS wallet addresses (Squad multisig accounts)
// export const BOSS = new PublicKey("45YnzauhsBM8CpUz96Djf8UG5vqq2Dua62wuW9H3jaJ5"); // WARN: SQUAD MAIN ACCOUNT!!!
// export const BOSS = new PublicKey("7rzEKejyAXJXMkGfRhMV9Vg1k7tFznBBEFu3sfLNz8LC"); // DEV Squad
export const BOSS = new PublicKey("EVdiVScB7LX1P3bn7ZLmLJTBrSSgRXPqRU3bVxrEpRb5"); // devnet Squad
// Note: In production, the actual boss is fetched from the program state, these are just for reference

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
        [this.statePda] = PublicKey.findProgramAddressSync([Buffer.from("state")], PROGRAM_ID);

        this.pdas = {
            offerVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("offer_vault_authority")], PROGRAM_ID)[0],
            permissionlessVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], PROGRAM_ID)[0],
            mintAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("mint_authority")], PROGRAM_ID)[0]
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

    // Transaction builders - return unsigned transactions for signing
    async buildMakeOfferIx(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        feeBasisPoints?: number;
        needsApproval?: boolean;
        allowPermissionless?: boolean;
        tokenInProgram?: PublicKey;
    }) {
        const feeBasisPoints = params.feeBasisPoints ?? 0;
        const needsApproval = params.needsApproval ?? false;
        const allowPermissionless = params.allowPermissionless ?? false;

        return await this.program.methods
            .makeOffer(feeBasisPoints, needsApproval, allowPermissionless)
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutMint: params.tokenOutMint
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
    }) {
        return await this.program.methods
            .addOfferVector(
                new BN(params.baseTime),
                new BN(params.basePrice),
                new BN(params.apr),
                new BN(params.priceFixDuration)
            )
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint
            })
            .instruction();
    }

    async buildCloseOfferIx(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
    }) {
        return await this.program.methods
            .closeOffer()
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint
            })
            .instruction();
    }

    async buildUpdateOfferFeeIx(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        newFeeBasisPoints: number;
    }) {
        return await this.program.methods
            .updateOfferFee(params.newFeeBasisPoints)
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint
            })
            .instruction();
    }

    async buildDeleteOfferVectorIx(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        vectorStartTimestamp: number;
    }) {
        return await this.program.methods
            .deleteOfferVector(new BN(params.vectorStartTimestamp))
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint
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
    }) {
        return await this.program.methods
            .takeOffer(new BN(params.tokenInAmount), params.approvalMessage ?? null)
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID
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
    }) {
        return await this.program.methods
            .takeOfferPermissionless(new BN(params.tokenInAmount), params.approvalMessage ?? null)
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
                boss: BOSS
            })
            .instruction();
    }

    async buildOfferVaultDepositIx(params: {
        amount: number,
        tokenMint: PublicKey,
        tokenProgram?: PublicKey,
    }) {
        return await this.program.methods
            .offerVaultDeposit(new BN(params.amount))
            .accounts({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            })
            .instruction();
    }

    async buildOfferVaultWithdrawIx(params: {
        amount: number,
        tokenMint: PublicKey,
        tokenProgram?: PublicKey,
    }) {
        return await this.program.methods
            .offerVaultWithdraw(new BN(params.amount))
            .accounts({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            })
            .instruction();
    }

    async buildAddAdminIx(params: { admin: PublicKey }) {
        return await this.program.methods
            .addAdmin(params.admin)
            .instruction();
    }

    async buildRemoveAdminIx(params: { admin: PublicKey }) {
        return await this.program.methods
            .removeAdmin(params.admin)
            .instruction();
    }

    async buildSetBossIx(params: { newBoss: PublicKey }) {
        return await this.program.methods
            .setBoss(params.newBoss)
            .instruction();
    }

    async buildSetKillSwitchIx(params: { enable: boolean }) {
        return await this.program.methods
            .setKillSwitch(params.enable)
            .instruction();
    }

    async buildInitializeIx() {
        return await this.program.methods
            .initialize()
            .instruction();
    }

    async buildMigrateStateIx() {
        return await this.program.methods
            .migrateV3()
            .accounts({
                state: this.statePda,
                permissionlessAuthority: this.pdas.permissionlessVaultAuthorityPda
            })
            .instruction();
    }

    async buildInitializeVaultAuthorityIx() {
        return await this.program.methods
            .initializeVaultAuthority()
            .instruction();
    }

    async buildInitializeMintAuthorityIx() {
        return await this.program.methods
            .initializeMintAuthority()
            .instruction();
    }

    async buildInitializePermissionlessAuthorityIx(params: { name: string }) {
        return await this.program.methods
            .initializePermissionlessAuthority(params.name)
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
