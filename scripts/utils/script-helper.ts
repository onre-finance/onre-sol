import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, Program } from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { BN } from "bn.js";
import { Onreapp } from "../../target/types/onreapp";
import idl from "../../target/idl/onreapp.json";
import bs58 from "bs58";

// Environment configuration
export const RPC_URL = process.env.SOL_MAINNET_RPC_URL || "https://api.mainnet-beta.solana.com";

// Program IDs
export const PROGRAM_ID = new PublicKey("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe"); // PROD
// export const PROGRAM_ID = new PublicKey("J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2"); // TEST
// export const PROGRAM_ID = new PublicKey("devHfQHgiFNifkLW49RCXpyTUZMyKuBNnFSbrQ8XsbX"); // DEV

// BOSS wallet addresses (Squad multisig accounts)
export const BOSS = new PublicKey("45YnzauhsBM8CpUz96Djf8UG5vqq2Dua62wuW9H3jaJ5"); // WARN: SQUAD MAIN ACCOUNT!!!
// export const BOSS = new PublicKey("7rzEKejyAXJXMkGfRhMV9Vg1k7tFznBBEFu3sfLNz8LC"); // DEV Squad
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
        offerAccountPda: PublicKey;
        offerVaultAuthorityPda: PublicKey;
        permissionlessVaultAuthorityPda: PublicKey;
        mintAuthorityPda: PublicKey;
    };

    private constructor(program: Program<Onreapp>, connection: Connection) {
        this.program = program;
        this.connection = connection;
        [this.statePda] = PublicKey.findProgramAddressSync([Buffer.from("state")], PROGRAM_ID);

        this.pdas = {
            offerAccountPda: PublicKey.findProgramAddressSync([Buffer.from("offers")], PROGRAM_ID)[0],
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

    async getOfferAccount() {
        return await this.program.account.offerAccount.fetch(this.pdas.offerAccountPda);
    }

    async getOffer(offerId: number) {
        const offerAccount = await this.getOfferAccount();
        return offerAccount.offers.find(offer => offer.offerId.toNumber() === offerId);
    }

    async getState() {
        return await this.program.account.state.fetch(this.statePda);
    }

    // Transaction builders - return unsigned transactions for signing
    async buildMakeOfferTransaction(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        feeBasisPoints?: number;
        tokenInProgram?: PublicKey;
        boss?: PublicKey;
    }) {
        const feeBasisPoints = params.feeBasisPoints ?? 0;

        const tx = await this.program.methods
            .makeOffer(new BN(feeBasisPoints))
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutMint: params.tokenOutMint,
                state: this.statePda,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildAddOfferVectorTransaction(params: {
        offerId: number,
        startTime: number,
        startPrice: number,
        apr: number,
        priceFixDuration: number,
        boss?: PublicKey;
    }) {
        const tx = await this.program.methods
            .addOfferVector(
                new BN(params.offerId),
                new BN(params.startTime),
                new BN(params.startPrice),
                new BN(params.apr),
                new BN(params.priceFixDuration)
            )
            .accountsPartial({
                state: this.statePda,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildCloseOfferTransaction(params: { offerId: number, boss?: PublicKey }) {
        const tx = await this.program.methods
            .closeOffer(new BN(params.offerId))
            .accountsPartial({
                state: this.statePda,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildUpdateOfferFeeTransaction(params: { offerId: number, newFee: number, boss?: PublicKey }) {
        const tx = await this.program.methods
            .updateOfferFee(new BN(params.offerId), new BN(params.newFee))
            .accountsPartial({
                state: this.statePda,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildDeleteOfferVectorTransaction(params: { offerId: number, vectorId: number, boss?: PublicKey }) {
        const tx = await this.program.methods
            .deleteOfferVector(new BN(params.offerId), new BN(params.vectorId))
            .accountsPartial({
                state: this.statePda,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildTakeOfferTransaction(params: {
        offerId: number,
        tokenInAmount: number,
        tokenInMint: PublicKey,
        tokenOutMint: PublicKey,
        user: PublicKey,
        tokenInProgram?: PublicKey,
        tokenOutProgram?: PublicKey,
        boss?: PublicKey
    }) {
        const tx = await this.program.methods
            .takeOffer(new BN(params.offerId), new BN(params.tokenInAmount))
            .accountsPartial({
                state: this.statePda,
                boss: params.boss ?? BOSS,
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildTakeOfferPermissionlessTransaction(params: {
        offerId: number,
        tokenInAmount: number,
        tokenInMint: PublicKey,
        tokenOutMint: PublicKey,
        user: PublicKey,
        tokenInProgram?: PublicKey,
        tokenOutProgram?: PublicKey,
        boss?: PublicKey
    }) {
        const tx = await this.program.methods
            .takeOfferPermissionless(new BN(params.offerId), new BN(params.tokenInAmount))
            .accountsPartial({
                state: this.statePda,
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildOfferVaultDepositTransaction(params: {
        amount: number,
        tokenMint: PublicKey,
        tokenProgram?: PublicKey,
        boss?: PublicKey
    }) {
        const tx = await this.program.methods
            .offerVaultDeposit(new BN(params.amount))
            .accountsPartial({
                state: this.statePda,
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildOfferVaultWithdrawTransaction(params: {
        amount: number,
        tokenMint: PublicKey,
        tokenProgram?: PublicKey,
        boss?: PublicKey
    }) {
        const tx = await this.program.methods
            .offerVaultWithdraw(new BN(params.amount))
            .accountsPartial({
                state: this.statePda,
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildAddAdminTransaction(params: { admin: PublicKey, boss?: PublicKey }) {
        const tx = await this.program.methods
            .addAdmin(params.admin)
            .accountsPartial({
                state: this.statePda,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildRemoveAdminTransaction(params: { admin: PublicKey, boss?: PublicKey }) {
        const tx = await this.program.methods
            .removeAdmin(params.admin)
            .accountsPartial({
                state: this.statePda,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildSetBossTransaction(params: { newBoss: PublicKey, boss?: PublicKey }) {
        const tx = await this.program.methods
            .setBoss(params.newBoss)
            .accountsPartial({
                state: this.statePda,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildSetKillSwitchTransaction(params: { enable: boolean, boss?: PublicKey }) {
        const tx = await this.program.methods
            .setKillSwitch(params.enable)
            .accounts({})
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildMigrateStateTransaction(params: { boss?: PublicKey } = {}) {
        const tx = await this.program.methods
            .migrateState()
            .accountsPartial({
                state: this.statePda,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    async buildInitializeVaultAuthorityTransaction(params: { boss?: PublicKey } = {}) {
        const tx = await this.program.methods
            .initializeVaultAuthority()
            .accountsPartial({
                state: this.statePda,
                boss: params.boss ?? BOSS
            })
            .transaction();

        return this.prepareTransaction(tx, params.boss);
    }

    // Helper to prepare transaction with boss as fee payer and recent blockhash
    async prepareTransaction(tx: Transaction, boss?: PublicKey) {
        const feePayer = boss ?? BOSS;
        tx.feePayer = feePayer;
        tx.recentBlockhash = (await this.connection.getLatestBlockhash()).blockhash;
        return tx;
    }

    /**
     * Serialize transaction to base58 for external signing
     */
    serializeTransaction(tx: Transaction): string {
        const serializedTx = tx.serialize({
            requireAllSignatures: false,
            verifySignatures: false,
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
