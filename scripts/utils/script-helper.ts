import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, Program, Wallet } from "@coral-xyz/anchor";
import BN from "bn.js";

import * as fs from "node:fs";
import * as os from "node:os";
import { Connection, Keypair, PublicKey, Transaction, TransactionInstruction } from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccountIdempotentInstruction, getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Onreapp } from "../../target/types/onreapp";
import idl from "../../target/idl/onreapp.json";
import bs58 from "bs58";

// Load .env file if present
import "./load-env";

// Import network configuration
import { getNetworkConfig, NETWORK_CONFIGS, NetworkConfig, printConfigSummary } from "./network-config";
import chalk from "chalk";

// ============================================================
// ACTIVE CONFIGURATION
// ============================================================

/**
 * Active network configuration - determined by NETWORK env variable.
 * Usage: NETWORK=mainnet-test tsx scripts/your-script.ts
 *
 * Available networks: mainnet-prod, mainnet-test, mainnet-dev, devnet-test, devnet-dev
 */
export const config = getNetworkConfig();
export const USDC_MINT = config.mints.usdc;
export const ONYC_MINT = config.mints.onyc;

// Re-export for convenience
export type { NetworkConfig };
export { NETWORK_CONFIGS, printConfigSummary };

/**
 * Helper class for Onre scripts - provides clean abstraction similar to test OnreProgram
 * Encapsulates common functionality to reduce duplication across scripts
 */
export class ScriptHelper {
    program: Program<Onreapp>;
    connection: Connection;
    statePda: PublicKey;
    networkConfig: NetworkConfig;
    wallet: Wallet;
    walletSource?: string;
    walletKeypair?: Keypair;

    pdas: {
        offerVaultAuthorityPda: PublicKey;
        permissionlessVaultAuthorityPda: PublicKey;
        mintAuthorityPda: PublicKey;
        bufferStatePda: PublicKey;
        bufferVaultAuthorityPda: PublicKey;
        managementFeeVaultAuthorityPda: PublicKey;
        performanceFeeVaultAuthorityPda: PublicKey;
        redemptionVaultAuthorityPda: PublicKey;
        marketStatsPda: PublicKey;
    };

    private constructor(program: Program<Onreapp>, connection: Connection, networkConfig: NetworkConfig, wallet: Wallet, walletSource?: string) {
        this.program = program;
        this.connection = connection;
        this.networkConfig = networkConfig;
        this.wallet = wallet;
        this.walletSource = walletSource;
        this.walletKeypair = (wallet as Wallet & { payer?: Keypair }).payer;
        [this.statePda] = PublicKey.findProgramAddressSync([Buffer.from("state")], program.programId);

        this.pdas = {
            offerVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("offer_vault_authority")], program.programId)[0],
            permissionlessVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], program.programId)[0],
            mintAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("mint_authority")], program.programId)[0],
            bufferStatePda: PublicKey.findProgramAddressSync([Buffer.from("buffer_state")], program.programId)[0],
            bufferVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("buffer_vault_authority")], program.programId)[0],
            managementFeeVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("management_fee_vault_authority")], program.programId)[0],
            performanceFeeVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("performance_fee_vault_authority")], program.programId)[0],
            redemptionVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("redemption_offer_vault_authority")], program.programId)[0],
            marketStatsPda: PublicKey.findProgramAddressSync([Buffer.from("market_stats")], program.programId)[0],
        };
    }

    /**
     * Create IDL with correct program ID for the active network
     */
    private static getIdlWithProgramId(): Onreapp {
        const idlCopy = JSON.parse(JSON.stringify(idl));
        idlCopy.address = config.programId.toBase58();
        return idlCopy as Onreapp;
    }

    /**
     * Create a ScriptHelper instance.
     *
     * @param walletPath - Optional wallet path:
     *   - undefined: tries Solana CLI default, falls back to random keypair
     *   - string with "/": uses as absolute/relative path
     *   - string without "/": looks for {name}.json in ~/.config/solana/
     */
    static async create(walletPath?: string): Promise<ScriptHelper> {
        const connection = new Connection(config.rpcUrl);

        let wallet: Wallet;
        let walletSource: string;

        if (walletPath) {
            // Custom path provided
            let keypairPath: string;
            if (walletPath.includes("/") || walletPath.includes("\\")) {
                keypairPath = walletPath.replace(/^~/, os.homedir());
            } else {
                keypairPath = `${os.homedir()}/.config/solana/${walletPath}.json`;
            }
            const keypairData = JSON.parse(fs.readFileSync(keypairPath, "utf-8"));
            const keypair = Keypair.fromSecretKey(new Uint8Array(keypairData));
            wallet = new Wallet(keypair);
            walletSource = keypairPath;
        } else {
            // Try Solana CLI default, fall back to random
            const cliKeypairPath = ScriptHelper.getSolanaCliKeypairPath();
            if (cliKeypairPath && fs.existsSync(cliKeypairPath)) {
                const keypairData = JSON.parse(fs.readFileSync(cliKeypairPath, "utf-8"));
                const keypair = Keypair.fromSecretKey(new Uint8Array(keypairData));
                wallet = new Wallet(keypair);
                walletSource = cliKeypairPath;
            } else {
                const keypair = Keypair.generate();
                wallet = new Wallet(keypair);
                walletSource = "generated (read-only)";
            }
        }

        const provider = new AnchorProvider(connection, wallet);
        const program = new Program<Onreapp>(ScriptHelper.getIdlWithProgramId(), provider);

        anchor.setProvider(provider);

        console.log(chalk.whiteBright(`Wallet:  ${wallet.publicKey.toBase58()} (${walletSource})\n`));

        return new ScriptHelper(program, connection, config, wallet, walletSource);
    }

    /**
     * Get the default keypair path from Solana CLI config (~/.config/solana/cli/config.yml)
     */
    private static getSolanaCliKeypairPath(): string | null {
        const configPath = `${os.homedir()}/.config/solana/cli/config.yml`;
        try {
            const configContent = fs.readFileSync(configPath, "utf-8");
            const match = configContent.match(/keypair_path:\s*(.+)/);
            if (match && match[1]) {
                return match[1].trim().replace(/^~/, os.homedir());
            }
        } catch {
            // Config file doesn't exist or can't be read
        }
        return null;
    }

    // Account getters
    async getBoss(): Promise<PublicKey> {
        const stateAccount = await this.program.account.state.fetch(this.statePda);
        return stateAccount.boss;
    }

    getOfferPda(tokenInMint: PublicKey, tokenOutMint: PublicKey): PublicKey {
        return PublicKey.findProgramAddressSync([Buffer.from("offer"), tokenInMint.toBuffer(), tokenOutMint.toBuffer()], this.program.programId)[0];
    }

    async getOffer(tokenInMint: PublicKey, tokenOutMint: PublicKey) {
        const offerPda = this.getOfferPda(tokenInMint, tokenOutMint);
        console.log(`Offer PDA: ${offerPda}`);
        return await this.program.account.offer.fetch(offerPda);
    }

    async getState() {
        return await this.program.account.state.fetch(this.statePda);
    }

    async getBufferState() {
        return await this.program.account.bufferState.fetch(this.pdas.bufferStatePda);
    }

    getBufferVaultAta(onycMint: PublicKey): PublicKey {
        return getAssociatedTokenAddressSync(onycMint, this.pdas.bufferVaultAuthorityPda, true, TOKEN_PROGRAM_ID);
    }

    getManagementFeeVaultAta(onycMint: PublicKey): PublicKey {
        return getAssociatedTokenAddressSync(onycMint, this.pdas.managementFeeVaultAuthorityPda, true, TOKEN_PROGRAM_ID);
    }

    getPerformanceFeeVaultAta(onycMint: PublicKey): PublicKey {
        return getAssociatedTokenAddressSync(onycMint, this.pdas.performanceFeeVaultAuthorityPda, true, TOKEN_PROGRAM_ID);
    }

    getMarketStatsPda(): PublicKey {
        return this.pdas.marketStatsPda;
    }

    async getMarketStats() {
        return await this.program.account.marketStats.fetch(this.pdas.marketStatsPda);
    }

    getRedemptionOfferPda(tokenInMint: PublicKey, tokenOutMint: PublicKey): PublicKey {
        return PublicKey.findProgramAddressSync([Buffer.from("redemption_offer"), tokenInMint.toBuffer(), tokenOutMint.toBuffer()], this.program.programId)[0];
    }

    getRedemptionRequestPda(redemptionOffer: PublicKey, counter: number): PublicKey {
        return PublicKey.findProgramAddressSync(
            [Buffer.from("redemption_request"), redemptionOffer.toBuffer(), new BN(counter).toArrayLike(Buffer, "le", 8)],
            this.program.programId,
        )[0];
    }

    async fetchRedemptionOffer(tokenInMint: PublicKey, tokenOutMint: PublicKey) {
        const pda = this.getRedemptionOfferPda(tokenInMint, tokenOutMint);
        return await this.program.account.redemptionOffer.fetch(pda);
    }

    async fetchRedemptionRequest(redemptionOffer: PublicKey, counter: number) {
        const pda = this.getRedemptionRequestPda(redemptionOffer, counter);
        return await this.program.account.redemptionRequest.fetch(pda);
    }

    /**
     * Create instructions for permissionless token accounts if they don't exist
     * Returns an array of instructions (may be empty if accounts already exist)
     */
    async buildCreatePermissionlessTokenAccountsIxs(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        tokenInProgram: PublicKey;
        tokenOutProgram: PublicKey;
        payer: PublicKey;
    }): Promise<TransactionInstruction[]> {
        const instructions: TransactionInstruction[] = [];
        const permissionlessAuthority = this.pdas.permissionlessVaultAuthorityPda;
        const payer = params.payer;

        // Create permissionless token_in account if it doesn't exist
        const permissionlessTokenInAccount = getAssociatedTokenAddressSync(params.tokenInMint, permissionlessAuthority, true, params.tokenInProgram);

        const tokenInAccountInfo = await this.connection.getAccountInfo(permissionlessTokenInAccount);
        if (!tokenInAccountInfo) {
            const createTokenInIx = createAssociatedTokenAccountIdempotentInstruction(
                payer,
                permissionlessTokenInAccount,
                permissionlessAuthority,
                params.tokenInMint,
                params.tokenInProgram,
            );
            instructions.push(createTokenInIx);
        }

        // Create permissionless token_out account if it doesn't exist
        const permissionlessTokenOutAccount = getAssociatedTokenAddressSync(params.tokenOutMint, permissionlessAuthority, true, params.tokenOutProgram);

        const tokenOutAccountInfo = await this.connection.getAccountInfo(permissionlessTokenOutAccount);
        if (!tokenOutAccountInfo) {
            const createTokenOutIx = createAssociatedTokenAccountIdempotentInstruction(
                payer,
                permissionlessTokenOutAccount,
                permissionlessAuthority,
                params.tokenOutMint,
                params.tokenOutProgram,
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
        boss: PublicKey;
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
                boss: params.boss,
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
        boss: PublicKey;
    }) {
        return await this.program.methods
            .addOfferVector(null, new BN(params.baseTime), new BN(params.basePrice), new BN(params.apr), new BN(params.priceFixDuration))
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                boss: params.boss,
            })
            .instruction();
    }

    async buildUpdateOfferFeeIx(params: { tokenInMint: PublicKey; tokenOutMint: PublicKey; newFeeBasisPoints: number; boss: PublicKey }) {
        return await this.program.methods
            .updateOfferFee(params.newFeeBasisPoints)
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                boss: params.boss,
            })
            .instruction();
    }

    async buildDeleteOfferVectorIx(params: { tokenInMint: PublicKey; tokenOutMint: PublicKey; vectorStartTimestamp: number; boss: PublicKey }) {
        return await this.program.methods
            .deleteOfferVector(new BN(params.vectorStartTimestamp))
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                boss: params.boss,
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
            .takeOffer(new BN(params.tokenInAmount), null)
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
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
        const vaultAuthority = this.pdas.offerVaultAuthorityPda;
        const permissionlessAuthority = this.pdas.permissionlessVaultAuthorityPda;
        const mintAuthority = this.pdas.mintAuthorityPda;

        return await this.program.methods
            .takeOfferPermissionlessExtended(new BN(params.tokenInAmount), null)
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                vaultAuthority,
                permissionlessAuthority,
                mintAuthority,
                marketStats: this.pdas.marketStatsPda,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    async buildOfferVaultDepositIx(params: {
        amount: number,
        tokenMint: PublicKey,
        tokenProgram?: PublicKey,
        depositor: PublicKey;
    }) {
        return await this.program.methods
            .offerVaultDeposit(new BN(params.amount))
            .accountsPartial({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
                depositor: params.depositor
            })
            .instruction();
    }

    async buildOfferVaultWithdrawIx(params: { amount: number; tokenMint: PublicKey; tokenProgram?: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .offerVaultWithdraw(new BN(params.amount))
            .accountsPartial({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
                boss: params.boss,
            })
            .instruction();
    }

    async buildRedemptionVaultDepositIx(params: { amount: number; tokenMint: PublicKey; tokenProgram?: PublicKey; depositor: PublicKey }) {
        return await this.program.methods
            .redemptionVaultDeposit(new BN(params.amount))
            .accountsPartial({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
                depositor: params.depositor
            })
            .instruction();
    }

    async buildRedemptionVaultWithdrawIx(params: { amount: number; tokenMint: PublicKey; tokenProgram?: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .redemptionVaultWithdraw(new BN(params.amount))
            .accountsPartial({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
                boss: params.boss,
            })
            .instruction();
    }

    async buildSetRedemptionAdminIx(params: { redemptionAdmin: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .setRedemptionAdmin(params.redemptionAdmin)
            .accountsPartial({
                boss: params.boss,
            })
            .instruction();
    }

    async buildInitializeBufferIx(params: { offer: PublicKey; onycMint: PublicKey; boss: PublicKey }) {
        const builder = this.program.methods
            .initializeBuffer()
            .accountsPartial({
                boss: params.boss,
                offer: params.offer,
                onycMint: params.onycMint,
                bufferState: this.pdas.bufferStatePda,
                bufferVaultAuthority: this.pdas.bufferVaultAuthorityPda,
                bufferVaultOnycAccount: this.getBufferVaultAta(params.onycMint),
                managementFeeVaultAuthority: this.pdas.managementFeeVaultAuthorityPda,
                managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.onycMint),
                performanceFeeVaultAuthority: this.pdas.performanceFeeVaultAuthorityPda,
                performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.onycMint),
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            });
        return await builder.instruction();
    }

    async buildSetBufferMainOfferIx(params: { offer: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .setMainOffer()
            .accountsPartial({
                offer: params.offer,
            })
            .instruction();
    }

    async buildSetBufferGrossYieldIx(params: { grossYield: number; boss: PublicKey }) {
        return await this.program.methods
            .setBufferGrossApr(new BN(params.grossYield))
            .accountsPartial({
                bufferState: this.pdas.bufferStatePda,
                boss: params.boss,
            })
            .instruction();
    }

    async buildSetBufferFeeConfigIx(params: {
        managementFeeBasisPoints: number;
        managementFeeWallet: PublicKey;
        performanceFeeBasisPoints: number;
        performanceFeeWallet: PublicKey;
        boss: PublicKey;
    }) {
        return await this.program.methods
            .setBufferFeeConfig(
                params.managementFeeBasisPoints,
                params.managementFeeWallet,
                params.performanceFeeBasisPoints,
                params.performanceFeeWallet,
            )
            .accountsPartial({
                bufferState: this.pdas.bufferStatePda,
                boss: params.boss,
            })
            .instruction();
    }

    async buildManageBufferIx(params: { offer: PublicKey; onycMint: PublicKey }) {
        return await this.program.methods
            .manageBuffer()
            .accountsPartial({
                bufferState: this.pdas.bufferStatePda,
                onycMint: params.onycMint,
                offer: params.offer,
                bufferVaultAuthority: this.pdas.bufferVaultAuthorityPda,
                bufferVaultOnycAccount: this.getBufferVaultAta(params.onycMint),
                managementFeeVaultAuthority: this.pdas.managementFeeVaultAuthorityPda,
                managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.onycMint),
                performanceFeeVaultAuthority: this.pdas.performanceFeeVaultAuthorityPda,
                performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.onycMint),
                mintAuthority: this.pdas.mintAuthorityPda,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    async buildBurnForNavIncreaseIx(params: { tokenInMint: PublicKey; onycMint: PublicKey; assetAdjustmentAmount: number; targetNav: number; boss: PublicKey }) {
        return await this.program.methods
            .burnForNavIncrease(new BN(params.assetAdjustmentAmount), new BN(params.targetNav))
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                bufferState: this.pdas.bufferStatePda,
                onycMint: params.onycMint,
                boss: params.boss,
                offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
                vaultTokenOutAccount: getAssociatedTokenAddressSync(params.onycMint, this.pdas.offerVaultAuthorityPda, true, TOKEN_PROGRAM_ID),
                bufferVaultAuthority: this.pdas.bufferVaultAuthorityPda,
                bufferVaultOnycAccount: this.getBufferVaultAta(params.onycMint),
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    async buildWithdrawManagementFeesIx(params: { boss: PublicKey; onycMint: PublicKey; amount: number }) {
        return await this.program.methods
            .withdrawManagementFees(new BN(params.amount))
            .accountsPartial({
                bufferState: this.pdas.bufferStatePda,
                managementFeeVaultAuthority: this.pdas.managementFeeVaultAuthorityPda,
                managementFeeRecipient: params.boss,
                onycMint: params.onycMint,
                managementFeeRecipientOnycAccount: getAssociatedTokenAddressSync(params.onycMint, params.boss, false, TOKEN_PROGRAM_ID),
                managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.onycMint),
                boss: params.boss,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    async buildWithdrawPerformanceFeesIx(params: { boss: PublicKey; onycMint: PublicKey; amount: number }) {
        return await this.program.methods
            .withdrawPerformanceFees(new BN(params.amount))
            .accountsPartial({
                bufferState: this.pdas.bufferStatePda,
                performanceFeeVaultAuthority: this.pdas.performanceFeeVaultAuthorityPda,
                performanceFeeRecipient: params.boss,
                onycMint: params.onycMint,
                performanceFeeRecipientOnycAccount: getAssociatedTokenAddressSync(params.onycMint, params.boss, false, TOKEN_PROGRAM_ID),
                performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.onycMint),
                boss: params.boss,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    async buildDeleteAllOfferVectorsIx(params: { tokenInMint: PublicKey; tokenOutMint: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .deleteAllOfferVectors()
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                boss: params.boss,
            })
            .instruction();
    }

    async buildClearAdminsIx(params: { boss: PublicKey }) {
        return await this.program.methods
            .clearAdmins()
            .accountsPartial({
                boss: params.boss,
            })
            .instruction();
    }

    async buildAddAdminIx(params: { admin: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .addAdmin(params.admin)
            .accountsPartial({
                boss: params.boss,
            })
            .instruction();
    }

    async buildRemoveAdminIx(params: { admin: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .removeAdmin(params.admin)
            .accountsPartial({
                boss: params.boss,
            })
            .instruction();
    }

    async buildProposeBossIx(params: { newBoss: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .proposeBoss(params.newBoss)
            .accountsPartial({
                boss: params.boss,
            })
            .instruction();
    }

    async buildAcceptBossIx(params: { newBoss: PublicKey }) {
        return await this.program.methods
            .acceptBoss()
            .accountsPartial({
                newBoss: params.newBoss,
            })
            .instruction();
    }

    async buildSetKillSwitchIx(params: { enable: boolean; boss: PublicKey }) {
        return await this.program.methods
            .setKillSwitch(params.enable)
            .accountsPartial({
                signer: params.boss,
            })
            .instruction();
    }

    async buildCloseStateIx(params: { boss: PublicKey }) {
        return await this.program.methods
            .closeState()
            .accountsPartial({
                boss: params?.boss,
                state: this.statePda,
            })
            .instruction();
    }

    async buildConfigureMaxSupplyIx(params: { maxSupply: string; boss: PublicKey }) {
        return await this.program.methods
            .configureMaxSupply(new BN(params.maxSupply))
            .accountsPartial({
                boss: params.boss,
            })
            .instruction();
    }

    async buildAddApproverIx(params: { approver: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .addApprover(params.approver)
            .accountsPartial({
                boss: params.boss,
            })
            .instruction();
    }

    async buildRemoveApproverIx(params: { approver: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .removeApprover(params.approver)
            .accountsPartial({
                boss: params.boss,
            })
            .instruction();
    }

    async buildSetOnycMintIx(params: { onycMint: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .setOnycMint()
            .accountsPartial({
                boss: params.boss,
                onycMint: params.onycMint,
            })
            .instruction();
    }

    async buildInitializeIx(params: { boss: PublicKey; programData?: PublicKey; onycMint?: PublicKey }) {
        return await this.program.methods
            .initialize()
            .accountsPartial({
                boss: params.boss,
                program: this.networkConfig.programId,
                programData:
                    params?.programData ??
                    PublicKey.findProgramAddressSync([this.networkConfig.programId.toBuffer()], new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111"))[0],
                onycMint: params?.onycMint ?? this.networkConfig.mints.onyc,
            })
            .instruction();
    }

    async buildInitializePermissionlessAuthorityIx(params: { name: string; boss: PublicKey }) {
        return await this.program.methods
            .initializePermissionlessAuthority(params.name)
            .accountsPartial({
                boss: params.boss,
            })
            .instruction();
    }

    async buildTransferMintAuthorityToProgramIx(params: { mint: PublicKey; tokenProgram?: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .transferMintAuthorityToProgram()
            .accountsPartial({
                boss: params.boss,
                mint: params.mint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    async buildTransferMintAuthorityToBossIx(params: { mint: PublicKey; tokenProgram?: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .transferMintAuthorityToBoss()
            .accountsPartial({
                boss: params.boss,
                mint: params.mint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
            })
            .signers([])
            .instruction();
    }

    async buildMintToIx(params: { amount: number }) {
        const state = await this.program.account.state.fetch(this.statePda);
        const onycMint = state.onycMint as PublicKey;
        const offer = (state.mainOffer as PublicKey).equals(PublicKey.default)
            ? PublicKey.default
            : (state.mainOffer as PublicKey);

        return await this.program.methods
            .mintTo(new BN(params.amount))
            .accountsPartial({
                tokenProgram: TOKEN_PROGRAM_ID,
                offer,
                bufferAccounts: {
                    bufferState: this.pdas.bufferStatePda,
                    bufferVaultOnycAccount: getAssociatedTokenAddressSync(onycMint, this.pdas.bufferVaultAuthorityPda, true, TOKEN_PROGRAM_ID),
                    managementFeeVaultOnycAccount: getAssociatedTokenAddressSync(onycMint, this.pdas.managementFeeVaultAuthorityPda, true, TOKEN_PROGRAM_ID),
                    performanceFeeVaultOnycAccount: getAssociatedTokenAddressSync(onycMint, this.pdas.performanceFeeVaultAuthorityPda, true, TOKEN_PROGRAM_ID),
                },
                offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
                offerVaultOnycAccount: getAssociatedTokenAddressSync(onycMint, this.pdas.offerVaultAuthorityPda, true, TOKEN_PROGRAM_ID),
                marketStats: this.pdas.marketStatsPda,
            })
            .instruction();
    }

    async buildMakeRedemptionOfferIx(params: {
        tokenInMint: PublicKey;
        tokenInProgram: PublicKey;
        tokenOutMint: PublicKey;
        tokenOutProgram: PublicKey;
        feeBasisPoints: number;
        boss: PublicKey;
    }) {
        return await this.program.methods
            .makeRedemptionOffer(params.feeBasisPoints)
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenInProgram: params.tokenInProgram,
                tokenOutMint: params.tokenOutMint,
                tokenOutProgram: params.tokenOutProgram,
                signer: params.boss,
            })
            .instruction();
    }

    async buildCreateRedemptionRequestIx(params: { redemptionOfferPda: PublicKey; tokenInMint: PublicKey; amount: number; redeemer: PublicKey; tokenProgram?: PublicKey }) {
        // Fetch the redemption offer to get the counter for PDA derivation
        const redemptionOffer = await this.program.account.redemptionOffer.fetch(params.redemptionOfferPda);

        // Derive the redemption request PDA using the counter
        const [redemptionRequest] = PublicKey.findProgramAddressSync(
            [Buffer.from("redemption_request"), params.redemptionOfferPda.toBuffer(), Buffer.from(redemptionOffer.requestCounter.toArrayLike(Buffer, "le", 8))],
            this.program.programId,
        );

        // Get the redemption vault authority PDA
        const [redemptionVaultAuthority] = PublicKey.findProgramAddressSync([Buffer.from("redemption_offer_vault_authority")], this.program.programId);

        // Get associated token accounts
        const redeemerTokenAccount = getAssociatedTokenAddressSync(params.tokenInMint, params.redeemer, false, params.tokenProgram ?? TOKEN_PROGRAM_ID);

        const vaultTokenAccount = getAssociatedTokenAddressSync(
            params.tokenInMint,
            redemptionVaultAuthority,
            true, // Allow off-curve for PDA
            params.tokenProgram ?? TOKEN_PROGRAM_ID,
        );

        return await this.program.methods
            .createRedemptionRequest(new BN(params.amount))
            .accountsPartial({
                redemptionOffer: params.redemptionOfferPda,
                tokenInMint: params.tokenInMint,
                redeemer: params.redeemer,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    async buildFulfillRedemptionRequestIx(params: { redemptionOfferPda: PublicKey; redemptionRequestPda: PublicKey; redemptionAdmin: PublicKey; amount: BN }) {
        return await this.program.methods
            .fulfillRedemptionRequest(params.amount)
            .accountsPartial({
                redemptionOffer: params.redemptionOfferPda,
                redemptionRequest: params.redemptionRequestPda,
                redemptionAdmin: params.redemptionAdmin,
            })
            .instruction();
    }

    async buildCancelRedemptionRequestIx(params: { redemptionOfferPda: PublicKey; redemptionRequestPda: PublicKey; signer: PublicKey }) {
        return await this.program.methods
            .cancelRedemptionRequest()
            .accountsPartial({
                redemptionOffer: params.redemptionOfferPda,
                redemptionRequest: params.redemptionRequestPda,
                signer: params.signer,
            })
            .instruction();
    }

    async buildUpdateRedemptionOfferFeeIx(params: { redemptionOfferPda: PublicKey; newFeeBasisPoints: number; boss: PublicKey }) {
        return await this.program.methods
            .updateRedemptionOfferFee(params.newFeeBasisPoints)
            .accountsPartial({
                redemptionOffer: params.redemptionOfferPda,
                boss: params.boss,
            })
            .instruction();
    }

    async prepareTransactionMultipleIxs(params: { ixs: TransactionInstruction[]; payer: PublicKey }) {
        const tx = new Transaction();
        for (const ix of params.ixs) {
            tx.add(ix);
        }
        tx.feePayer = params.payer;
        tx.recentBlockhash = (await this.connection.getLatestBlockhash()).blockhash;
        return tx;
    }

    // Helper to prepare transaction with boss as fee payer and recent blockhash
    async prepareTransaction(params: { ix: TransactionInstruction; payer: PublicKey }) {
        return await this.prepareTransactionMultipleIxs({ ixs: [params.ix], payer: params.payer });
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
