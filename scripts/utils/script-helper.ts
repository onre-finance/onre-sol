import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, Program, Wallet } from "@coral-xyz/anchor";
import BN from "bn.js";

import * as fs from "node:fs";
import * as os from "node:os";
import { ComputeBudgetProgram, Connection, Keypair, PublicKey, SYSVAR_INSTRUCTIONS_PUBKEY, Transaction, TransactionInstruction } from "@solana/web3.js";
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

type RawAmount = string | number;

// Re-export for convenience
export type { NetworkConfig };
export { NETWORK_CONFIGS, printConfigSummary };

const CONFIGURABLE_VAULTS = {
    "offer-fee": { seed: "offer_fee", kind: { offerFee: {} } },
    "permissionless-offer-fee": { seed: "permissionless_offer_fee", kind: { permissionlessOfferFee: {} } },
    "redemption-fee": { seed: "redemption_fee", kind: { redemptionFee: {} } },
    "management-fee": { seed: "management_fee", kind: { managementFee: {} } },
    "performance-fee": { seed: "performance_fee", kind: { performanceFee: {} } },
    "prop-amm-buy-fee": { seed: "prop_amm_buy_fee", kind: { propAmmBuyFee: {} } },
    "prop-amm-sell-fee": { seed: "prop_amm_sell_fee", kind: { propAmmSellFee: {} } },
    "offer-proceeds": { seed: "offer_proceeds", kind: { offerProceeds: {} } },
    "prop-amm-proceeds": { seed: "prop_amm_proceeds", kind: { propAmmProceeds: {} } },
} as const;

export type ConfigurableVaultCliKind = keyof typeof CONFIGURABLE_VAULTS;

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
        reserveVaultAuthorityPda: PublicKey;
        managementFeeVaultPda: PublicKey;
        performanceFeeVaultPda: PublicKey;
        redemptionVaultAuthorityPda: PublicKey;
        marketStatsPda: PublicKey;
        circulatingSupplyExcludedBalancePda: PublicKey;
        circulatingSupplyExcludedAccountsPda: PublicKey;
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
            reserveVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("reserve_vault_authority")], program.programId)[0],
            managementFeeVaultPda: PublicKey.findProgramAddressSync([Buffer.from("configurable_vault"), Buffer.from("management_fee")], program.programId)[0],
            performanceFeeVaultPda: PublicKey.findProgramAddressSync([Buffer.from("configurable_vault"), Buffer.from("performance_fee")], program.programId)[0],
            redemptionVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("redemption_offer_vault_authority")], program.programId)[0],
            marketStatsPda: PublicKey.findProgramAddressSync([Buffer.from("market_stats")], program.programId)[0],
            circulatingSupplyExcludedBalancePda: PublicKey.findProgramAddressSync([Buffer.from("circ_supply_excl_balance")], program.programId)[0],
            circulatingSupplyExcludedAccountsPda: PublicKey.findProgramAddressSync([Buffer.from("circ_supply_excl_accounts")], program.programId)[0],
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
        return getAssociatedTokenAddressSync(onycMint, this.pdas.reserveVaultAuthorityPda, true, TOKEN_PROGRAM_ID);
    }

    getManagementFeeVaultAta(onycMint: PublicKey): PublicKey {
        return getAssociatedTokenAddressSync(onycMint, this.pdas.managementFeeVaultPda, true, TOKEN_PROGRAM_ID);
    }

    getPerformanceFeeVaultAta(onycMint: PublicKey): PublicKey {
        return getAssociatedTokenAddressSync(onycMint, this.pdas.performanceFeeVaultPda, true, TOKEN_PROGRAM_ID);
    }

    getMarketStatsPda(): PublicKey {
        return this.pdas.marketStatsPda;
    }

    getConfigurableVaultPda(kind: ConfigurableVaultCliKind): PublicKey {
        return PublicKey.findProgramAddressSync([Buffer.from("configurable_vault"), Buffer.from(CONFIGURABLE_VAULTS[kind].seed)], this.program.programId)[0];
    }

    getConfigurableVaultAta(kind: ConfigurableVaultCliKind, mint: PublicKey, tokenProgram: PublicKey = TOKEN_PROGRAM_ID): PublicKey {
        return getAssociatedTokenAddressSync(mint, this.getConfigurableVaultPda(kind), true, tokenProgram);
    }

    async getMarketStats() {
        return await this.program.account.marketStats.fetch(this.pdas.marketStatsPda);
    }

    getRedemptionOfferPda(tokenInMint: PublicKey, tokenOutMint: PublicKey): PublicKey {
        return PublicKey.findProgramAddressSync([Buffer.from("redemption_offer"), tokenInMint.toBuffer(), tokenOutMint.toBuffer()], this.program.programId)[0];
    }

    getPropAmmPairPda(offer: PublicKey): PublicKey {
        return PublicKey.findProgramAddressSync([Buffer.from("prop_amm_pair"), offer.toBuffer()], this.program.programId)[0];
    }

    getRedemptionRequestPda(redemptionOffer: PublicKey, redeemer: PublicKey, requestId: string): PublicKey {
        if (Buffer.byteLength(requestId, "utf8") !== 32) {
            throw new Error("Redemption request ID must be exactly 32 UTF-8 bytes (UUID without hyphens).");
        }
        return PublicKey.findProgramAddressSync(
            [Buffer.from("redemption_request"), redemptionOffer.toBuffer(), redeemer.toBuffer(), Buffer.from(requestId, "utf8")],
            this.program.programId,
        )[0];
    }

    async fetchRedemptionOffer(tokenInMint: PublicKey, tokenOutMint: PublicKey) {
        const pda = this.getRedemptionOfferPda(tokenInMint, tokenOutMint);
        return await this.program.account.redemptionOffer.fetch(pda);
    }

    async fetchRedemptionRequest(redemptionOffer: PublicKey, redeemer: PublicKey, requestId: string) {
        const pda = this.getRedemptionRequestPda(redemptionOffer, redeemer, requestId);
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

    async buildUpdateOfferPermissionlessFeeIx(params: { tokenInMint: PublicKey; tokenOutMint: PublicKey; newFeeBasisPointsPermissionless: number; boss: PublicKey }) {
        return await this.program.methods
            .updateOfferPermissionlessFee(params.newFeeBasisPointsPermissionless)
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                boss: params.boss,
            })
            .instruction();
    }

    async buildSetOfferDisabledIx(params: { tokenInMint: PublicKey; tokenOutMint: PublicKey; disabled: boolean; signer: PublicKey }) {
        return await this.program.methods
            .setOfferDisabled(params.disabled)
            .accountsPartial({
                offer: this.getOfferPda(params.tokenInMint, params.tokenOutMint),
                signer: params.signer,
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

    async buildTakeOfferLegacyIxs(params: {
        tokenInAmount: RawAmount;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        approvalMessage?: any;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        const tokenInProgram = params.tokenInProgram ?? TOKEN_PROGRAM_ID;
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;
        const boss = await this.getBoss();
        const bossTokenInAccount = getAssociatedTokenAddressSync(params.tokenInMint, boss, true, tokenInProgram);
        const createBossTokenInIx = createAssociatedTokenAccountIdempotentInstruction(params.user, bossTokenInAccount, boss, params.tokenInMint, tokenInProgram);

        const takeIx = await this.program.methods
            .takeOffer(new BN(params.tokenInAmount), params.approvalMessage ?? null)
            .accountsPartial({
                offer: this.getOfferPda(params.tokenInMint, params.tokenOutMint),
                state: this.statePda,
                boss,
                vaultAuthority: this.pdas.offerVaultAuthorityPda,
                vaultTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, this.pdas.offerVaultAuthorityPda, true, tokenInProgram),
                vaultTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, this.pdas.offerVaultAuthorityPda, true, tokenOutProgram),
                tokenInMint: params.tokenInMint,
                tokenInProgram,
                tokenOutMint: params.tokenOutMint,
                tokenOutProgram,
                userTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, params.user, false, tokenInProgram),
                userTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, params.user, false, tokenOutProgram),
                bossTokenInAccount,
                mintAuthority: this.pdas.mintAuthorityPda,
                instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
                user: params.user,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .instruction();

        return [createBossTokenInIx, takeIx];
    }

    async buildTakeOfferIx(params: {
        tokenInAmount: RawAmount;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        approvalMessage?: any;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        const tokenInProgram = params.tokenInProgram ?? TOKEN_PROGRAM_ID;
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;
        const state = await this.getState();

        return await this.program.methods
            .takeOfferV2(new BN(params.tokenInAmount), params.approvalMessage ?? null)
            .accountsPartial({
                offer: this.getOfferPda(params.tokenInMint, params.tokenOutMint),
                state: this.statePda,
                vaultAuthority: this.pdas.offerVaultAuthorityPda,
                vaultTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, this.pdas.offerVaultAuthorityPda, true, tokenInProgram),
                vaultTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, this.pdas.offerVaultAuthorityPda, true, tokenOutProgram),
                tokenInMint: params.tokenInMint,
                tokenInProgram,
                tokenOutMint: params.tokenOutMint,
                tokenOutProgram,
                user: params.user,
                userTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, params.user, false, tokenInProgram),
                userTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, params.user, false, tokenOutProgram),
                redemptionOffer: this.getRedemptionOfferPda(params.tokenOutMint, params.tokenInMint),
                redemptionVaultAuthority: this.pdas.redemptionVaultAuthorityPda,
                redemptionVaultTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, this.pdas.redemptionVaultAuthorityPda, true, tokenInProgram),
                offerProceedsVault: this.getConfigurableVaultPda("offer-proceeds"),
                offerProceedsTokenInAccount: this.getConfigurableVaultAta("offer-proceeds", params.tokenInMint, tokenInProgram),
                offerFeeVault: this.getConfigurableVaultPda("offer-fee"),
                offerFeeTokenInAccount: this.getConfigurableVaultAta("offer-fee", params.tokenInMint, tokenInProgram),
                mintAuthority: this.pdas.mintAuthorityPda,
                bufferAccounts: {
                    bufferState: this.pdas.bufferStatePda,
                    reserveVaultOnycAccount: this.getBufferVaultAta(params.tokenOutMint),
                    managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.tokenOutMint),
                    performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.tokenOutMint),
                },
                marketStats: this.pdas.marketStatsPda,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
                instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
                mainOffer: state.mainOffer as PublicKey,
            })
            .instruction();
    }

    async buildTakeOfferPermissionlessLegacyIxs(params: {
        tokenInAmount: RawAmount;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        approvalMessage?: any;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        const vaultAuthority = this.pdas.offerVaultAuthorityPda;
        const permissionlessAuthority = this.pdas.permissionlessVaultAuthorityPda;
        const tokenInProgram = params.tokenInProgram ?? TOKEN_PROGRAM_ID;
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;
        const boss = await this.getBoss();
        const bossTokenInAccount = getAssociatedTokenAddressSync(params.tokenInMint, boss, true, tokenInProgram);
        const permissionlessTokenInAccount = getAssociatedTokenAddressSync(params.tokenInMint, permissionlessAuthority, true, tokenInProgram);
        const permissionlessTokenOutAccount = getAssociatedTokenAddressSync(params.tokenOutMint, permissionlessAuthority, true, tokenOutProgram);

        const setupIxs = [
            createAssociatedTokenAccountIdempotentInstruction(params.user, bossTokenInAccount, boss, params.tokenInMint, tokenInProgram),
            createAssociatedTokenAccountIdempotentInstruction(params.user, permissionlessTokenInAccount, permissionlessAuthority, params.tokenInMint, tokenInProgram),
            createAssociatedTokenAccountIdempotentInstruction(params.user, permissionlessTokenOutAccount, permissionlessAuthority, params.tokenOutMint, tokenOutProgram),
        ];

        const takeIx = await this.program.methods
            .takeOfferPermissionless(new BN(params.tokenInAmount), params.approvalMessage ?? null)
            .accountsPartial({
                offer: this.getOfferPda(params.tokenInMint, params.tokenOutMint),
                state: this.statePda,
                boss,
                vaultAuthority,
                vaultTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, vaultAuthority, true, tokenInProgram),
                vaultTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, vaultAuthority, true, tokenOutProgram),
                permissionlessAuthority,
                permissionlessTokenInAccount,
                permissionlessTokenOutAccount,
                tokenInMint: params.tokenInMint,
                tokenInProgram,
                tokenOutMint: params.tokenOutMint,
                tokenOutProgram,
                userTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, params.user, false, tokenInProgram),
                userTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, params.user, false, tokenOutProgram),
                bossTokenInAccount,
                mintAuthority: this.pdas.mintAuthorityPda,
                instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
                user: params.user,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .instruction();

        return [...setupIxs, takeIx];
    }

    async buildTakeOfferPermissionlessIx(params: {
        tokenInAmount: RawAmount;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        const vaultAuthority = this.pdas.offerVaultAuthorityPda;
        const permissionlessAuthority = this.pdas.permissionlessVaultAuthorityPda;
        const mintAuthority = this.pdas.mintAuthorityPda;
        const tokenInProgram = params.tokenInProgram ?? TOKEN_PROGRAM_ID;
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;
        const state = await this.getState();

        return await this.program.methods
            .takeOfferPermissionlessV2(new BN(params.tokenInAmount))
            .accountsPartial({
                offer: this.getOfferPda(params.tokenInMint, params.tokenOutMint),
                state: this.statePda,
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                vaultAuthority,
                vaultTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, vaultAuthority, true, tokenInProgram),
                vaultTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, vaultAuthority, true, tokenOutProgram),
                permissionlessAuthority,
                permissionlessTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, permissionlessAuthority, true, tokenInProgram),
                permissionlessTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, permissionlessAuthority, true, tokenOutProgram),
                userTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, params.user, false, tokenInProgram),
                userTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, params.user, false, tokenOutProgram),
                redemptionOffer: this.getRedemptionOfferPda(params.tokenOutMint, params.tokenInMint),
                redemptionVaultAuthority: this.pdas.redemptionVaultAuthorityPda,
                redemptionVaultTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, this.pdas.redemptionVaultAuthorityPda, true, tokenInProgram),
                offerProceedsVault: this.getConfigurableVaultPda("offer-proceeds"),
                offerProceedsTokenInAccount: this.getConfigurableVaultAta("offer-proceeds", params.tokenInMint, tokenInProgram),
                permissionlessOfferFeeVault: this.getConfigurableVaultPda("permissionless-offer-fee"),
                permissionlessOfferFeeTokenInAccount: this.getConfigurableVaultAta("permissionless-offer-fee", params.tokenInMint, tokenInProgram),
                mintAuthority,
                bufferAccounts: {
                    bufferState: this.pdas.bufferStatePda,
                    reserveVaultOnycAccount: this.getBufferVaultAta(params.tokenOutMint),
                    managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.tokenOutMint),
                    performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.tokenOutMint),
                },
                marketStats: this.pdas.marketStatsPda,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
                tokenInProgram,
                tokenOutProgram,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
                mainOffer: state.mainOffer as PublicKey,
            })
            .instruction();
    }

    async buildOfferVaultDepositIx(params: { amount: number; tokenMint: PublicKey; tokenProgram?: PublicKey; depositor: PublicKey }) {
        return await this.program.methods
            .offerVaultDeposit(new BN(params.amount))
            .accountsPartial({
                state: this.statePda,
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
                depositor: params.depositor,
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
                state: this.statePda,
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
                depositor: params.depositor,
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

    async buildSetConfigurableVaultDestinationIx(params: { kind: ConfigurableVaultCliKind; destination: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .setConfigurableVaultDestination(CONFIGURABLE_VAULTS[params.kind].kind, params.destination)
            .accountsPartial({
                boss: params.boss,
                configurableVault: this.getConfigurableVaultPda(params.kind),
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .instruction();
    }

    async buildWithdrawConfigurableVaultIx(params: { kind: ConfigurableVaultCliKind; mint: PublicKey; amount: string; caller: PublicKey; tokenProgram?: PublicKey }) {
        const tokenProgram = params.tokenProgram ?? TOKEN_PROGRAM_ID;
        const configurableVault = this.getConfigurableVaultPda(params.kind);
        const vault = await this.program.account.configurableVault.fetch(configurableVault);
        const destination = vault.withdrawalDestination as PublicKey;
        if (destination.equals(PublicKey.default)) {
            throw new Error(`Configurable vault ${params.kind} has no withdrawal destination configured`);
        }

        return await this.program.methods
            .withdrawConfigurableVault(CONFIGURABLE_VAULTS[params.kind].kind, new BN(params.amount))
            .accountsPartial({
                caller: params.caller,
                configurableVault,
                vaultTokenAccount: this.getConfigurableVaultAta(params.kind, params.mint, tokenProgram),
                destination,
                destinationTokenAccount: getAssociatedTokenAddressSync(params.mint, destination, false, tokenProgram),
                mint: params.mint,
                tokenProgram,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .instruction();
    }

    async buildSetWorkerIx(params: { worker: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .setWorker(params.worker)
            .accountsPartial({
                boss: params.boss,
            })
            .instruction();
    }

    async buildSettleBufferIx(params: { worker: PublicKey }) {
        const state = await this.getState();
        const onycMint = state.onycMint as PublicKey;

        return await this.program.methods
            .settleBuffer()
            .accountsPartial({
                state: this.statePda,
                worker: params.worker,
                onycMint,
                mintAuthority: this.pdas.mintAuthorityPda,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
                mainOffer: state.mainOffer as PublicKey,
                bufferAccounts: {
                    bufferState: this.pdas.bufferStatePda,
                    reserveVaultOnycAccount: this.getBufferVaultAta(onycMint),
                    managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(onycMint),
                    performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(onycMint),
                },
                marketStats: this.pdas.marketStatsPda,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
            })
            .instruction();
    }

    async buildInitializeBufferIx(params: { offer: PublicKey; onycMint: PublicKey; boss: PublicKey }) {
        const builder = this.program.methods.initializeBuffer().accountsPartial({
            boss: params.boss,
            offer: params.offer,
            onycMint: params.onycMint,
            bufferState: this.pdas.bufferStatePda,
            reserveVaultAuthority: this.pdas.reserveVaultAuthorityPda,
            reserveVaultOnycAccount: this.getBufferVaultAta(params.onycMint),
            managementFeeVault: this.pdas.managementFeeVaultPda,
            managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.onycMint),
            performanceFeeVault: this.pdas.performanceFeeVaultPda,
            performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.onycMint),
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        });
        return await builder.instruction();
    }

    async buildSetBufferGrossYieldIx(params: { grossYield: number; boss: PublicKey }) {
        const state = await this.getState();
        const onycMint = state.onycMint as PublicKey;

        return await this.program.methods
            .setBufferGrossApr(new BN(params.grossYield))
            .accountsPartial({
                state: this.statePda,
                boss: params.boss,
                mainOffer: state.mainOffer as PublicKey,
                onycMint,
                offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
                mintAuthority: this.pdas.mintAuthorityPda,
                bufferAccounts: {
                    bufferState: this.pdas.bufferStatePda,
                    reserveVaultOnycAccount: this.getBufferVaultAta(onycMint),
                    managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(onycMint),
                    performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(onycMint),
                },
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
                marketStats: this.pdas.marketStatsPda,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
            })
            .instruction();
    }

    async buildSetBufferFeeConfigIx(params: {
        managementFeeBps: number;
        performanceFeeBps: number;
        performanceFeeHighWatermarkEnabled: boolean;
        boss: PublicKey;
    }) {
        const state = await this.getState();
        const onycMint = state.onycMint as PublicKey;

        return await this.program.methods
            .setBufferFeeConfig(params.managementFeeBps, params.performanceFeeBps, params.performanceFeeHighWatermarkEnabled)
            .accountsPartial({
                boss: params.boss,
                mainOffer: state.mainOffer as PublicKey,
                onycMint,
                offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
                mintAuthority: this.pdas.mintAuthorityPda,
                bufferAccounts: {
                    bufferState: this.pdas.bufferStatePda,
                    reserveVaultOnycAccount: this.getBufferVaultAta(onycMint),
                    managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(onycMint),
                    performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(onycMint),
                },
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
                marketStats: this.pdas.marketStatsPda,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
            })
            .instruction();
    }

    async buildDepositReserveVaultIx(params: { onycMint: PublicKey; amount: RawAmount; depositor: PublicKey; tokenProgram?: PublicKey }) {
        const tokenProgram = params.tokenProgram ?? TOKEN_PROGRAM_ID;
        return await this.program.methods
            .depositReserveVault(new BN(params.amount))
            .accountsPartial({
                bufferState: this.pdas.bufferStatePda,
                reserveVaultAuthority: this.pdas.reserveVaultAuthorityPda,
                onycMint: params.onycMint,
                depositorOnycAccount: getAssociatedTokenAddressSync(params.onycMint, params.depositor, false, tokenProgram),
                reserveVaultOnycAccount: getAssociatedTokenAddressSync(params.onycMint, this.pdas.reserveVaultAuthorityPda, true, tokenProgram),
                depositor: params.depositor,
                tokenProgram,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .instruction();
    }

    async buildWithdrawReserveVaultIx(params: { onycMint: PublicKey; amount: RawAmount; boss: PublicKey; tokenProgram?: PublicKey }) {
        const tokenProgram = params.tokenProgram ?? TOKEN_PROGRAM_ID;
        return await this.program.methods
            .withdrawReserveVault(new BN(params.amount))
            .accountsPartial({
                bufferState: this.pdas.bufferStatePda,
                reserveVaultAuthority: this.pdas.reserveVaultAuthorityPda,
                onycMint: params.onycMint,
                bossOnycAccount: getAssociatedTokenAddressSync(params.onycMint, params.boss, false, tokenProgram),
                reserveVaultOnycAccount: getAssociatedTokenAddressSync(params.onycMint, this.pdas.reserveVaultAuthorityPda, true, tokenProgram),
                boss: params.boss,
                tokenProgram,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .instruction();
    }

    async buildBurnForNavIncreaseIx(params: { onycMint: PublicKey; assetAdjustmentAmount: number; boss: PublicKey; mainOffer: PublicKey }) {
        return await this.program.methods
            .burnForNavIncrease(new BN(params.assetAdjustmentAmount))
            .accountsPartial({
                bufferState: this.pdas.bufferStatePda,
                onycMint: params.onycMint,
                boss: params.boss,
                mainOffer: params.mainOffer,
                offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
                reserveVaultAuthority: this.pdas.reserveVaultAuthorityPda,
                reserveVaultOnycAccount: this.getBufferVaultAta(params.onycMint),
                managementFeeVault: this.pdas.managementFeeVaultPda,
                managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.onycMint),
                performanceFeeVault: this.pdas.performanceFeeVaultPda,
                performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.onycMint),
                mintAuthority: this.pdas.mintAuthorityPda,
                marketStats: this.pdas.marketStatsPda,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
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

    async buildConfigureMaxMintAmountIx(params: { maxMintAmount: string; boss: PublicKey }) {
        return await this.program.methods
            .configureMaxMintAmount(new BN(params.maxMintAmount))
            .accountsPartial({
                boss: params.boss,
            })
            .instruction();
    }

    async buildSetMainOfferIx(params: { offer: PublicKey; boss: PublicKey }) {
        return await this.program.methods
            .setMainOffer()
            .accountsPartial({
                boss: params.boss,
                offer: params.offer,
            })
            .instruction();
    }

    async buildSetCirculatingSupplyExcludedAccountsIx(params: { owners: PublicKey[]; boss: PublicKey }) {
        const owners = [...params.owners];
        if (owners.length > 20) {
            throw new Error("At most 20 excluded owners can be configured");
        }
        return await this.program.methods
            .setCirculatingSupplyExcludedAccounts(owners)
            .accountsPartial({
                boss: params.boss,
                excludedAccounts: this.pdas.circulatingSupplyExcludedAccountsPda,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .instruction();
    }

    async buildUpdateCirculatingSupplyExcludedBalanceIx(params: { onycMint: PublicKey; signer: PublicKey; tokenProgram?: PublicKey }) {
        const excludedAccounts = await this.program.account.circulatingSupplyExcludedAccounts.fetch(this.pdas.circulatingSupplyExcludedAccountsPda).catch((error: any) => {
            const message = error?.message || String(error);
            if (message.includes("Account does not exist") || message.includes("AccountNotFound")) {
                throw new Error("Circulating supply excluded owners are not configured. Run `state set-excluded-owners` first.");
            }
            throw error;
        });
        const tokenProgram = params.tokenProgram ?? TOKEN_PROGRAM_ID;
        const remainingAccounts = excludedAccounts.owners
            .filter((owner: PublicKey) => !owner.equals(PublicKey.default))
            .map((owner: PublicKey) => ({
                pubkey: getAssociatedTokenAddressSync(params.onycMint, owner, false, tokenProgram),
                isWritable: false,
                isSigner: false,
            }));

        return await this.program.methods
            .updateCirculatingSupplyExcludedBalance()
            .accountsPartial({
                onycMint: params.onycMint,
                excludedAccounts: this.pdas.circulatingSupplyExcludedAccountsPda,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
                tokenProgram,
                signer: params.signer,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .remainingAccounts(remainingAccounts)
            .instruction();
    }

    async buildRefreshMarketStatsIx(params: { tokenInMint: PublicKey; signer: PublicKey }) {
        const state = await this.getState();
        return await this.program.methods
            .refreshMarketStats()
            .accountsPartial({
                mainOffer: state.mainOffer as PublicKey,
                tokenInMint: params.tokenInMint,
                state: this.statePda,
                onycMint: state.onycMint as PublicKey,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
                marketStats: this.pdas.marketStatsPda,
                signer: params.signer,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .instruction();
    }

    async buildConfigurePropAmmIx(params: {
        assetMint: PublicKey;
        enabled: boolean;
        curvePegHaircutBps: number;
        curveExponentScaled: number;
        cadenceThreshold: number;
        cadenceWaveScaled: number;
        epochDurationSeconds: string;
        wallSensitivityScaled: number;
        minimumSellHaircutOnyc: string;
        boss: PublicKey;
    }) {
        const state = await this.getState();
        const offer = this.getOfferPda(params.assetMint, state.onycMint as PublicKey);

        return await this.program.methods
            .configurePropAmm(
                params.enabled,
                params.curvePegHaircutBps,
                params.curveExponentScaled,
                params.cadenceThreshold,
                params.cadenceWaveScaled,
                new BN(params.epochDurationSeconds),
                params.wallSensitivityScaled,
                new BN(params.minimumSellHaircutOnyc),
            )
            .accountsPartial({
                state: this.statePda,
                offer,
                assetMint: params.assetMint,
                propAmmPairState: this.getPropAmmPairPda(offer),
                boss: params.boss,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .instruction();
    }

    async buildQuoteSwapBuyIx(params: { tokenInAmount: string; tokenInMint: PublicKey; tokenOutMint: PublicKey }) {
        const offer = this.getOfferPda(params.tokenInMint, params.tokenOutMint);
        return await this.program.methods
            .quoteSwapBuy(new BN(params.tokenInAmount))
            .accountsPartial({
                offer,
                propAmmPairState: this.getPropAmmPairPda(offer),
                state: this.statePda,
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
            })
            .instruction();
    }

    async buildQuoteSwapSellIx(params: { tokenInAmount: string; tokenInMint: PublicKey; tokenOutMint: PublicKey; tokenOutProgram?: PublicKey }) {
        const state = await this.getState();
        const onycMint = state.onycMint as PublicKey;
        if (!params.tokenInMint.equals(onycMint)) {
            throw new Error("Prop AMM sell quotes must use ONYC as token in");
        }

        const assetMint = params.tokenOutMint;
        const offer = this.getOfferPda(assetMint, onycMint);
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;

        return await this.program.methods
            .quoteSwapSell(new BN(params.tokenInAmount))
            .accountsPartial({
                offer,
                propAmmPairState: this.getPropAmmPairPda(offer),
                redemptionOffer: this.getRedemptionOfferPda(onycMint, assetMint),
                state: this.statePda,
                redemptionVaultAuthority: this.pdas.redemptionVaultAuthorityPda,
                redemptionVaultTokenOutAccount: getAssociatedTokenAddressSync(assetMint, this.pdas.redemptionVaultAuthorityPda, true, tokenOutProgram),
                tokenInMint: onycMint,
                tokenOutMint: assetMint,
                tokenOutProgram,
                marketStats: this.pdas.marketStatsPda,
            })
            .instruction();
    }

    async buildOpenSwapBuyIx(params: {
        tokenInAmount: string;
        minimumOut: string;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        const tokenInProgram = params.tokenInProgram ?? TOKEN_PROGRAM_ID;
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;
        const offer = this.getOfferPda(params.tokenInMint, params.tokenOutMint);
        const state = await this.getState();

        return await this.program.methods
            .openSwapBuy(new BN(params.tokenInAmount), new BN(params.minimumOut))
            .accountsPartial({
                offer,
                propAmmPairState: this.getPropAmmPairPda(offer),
                redemptionOffer: this.getRedemptionOfferPda(params.tokenOutMint, params.tokenInMint),
                state: this.statePda,
                offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
                redemptionVaultAuthority: this.pdas.redemptionVaultAuthorityPda,
                offerVaultTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, this.pdas.offerVaultAuthorityPda, true, tokenInProgram),
                offerVaultTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, this.pdas.offerVaultAuthorityPda, true, tokenOutProgram),
                redemptionVaultTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, this.pdas.redemptionVaultAuthorityPda, true, tokenInProgram),
                tokenInMint: params.tokenInMint,
                tokenInProgram,
                tokenOutMint: params.tokenOutMint,
                tokenOutProgram,
                userTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, params.user, false, tokenInProgram),
                userTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, params.user, false, tokenOutProgram),
                propAmmProceedsVault: this.getConfigurableVaultPda("prop-amm-proceeds"),
                propAmmProceedsTokenInAccount: this.getConfigurableVaultAta("prop-amm-proceeds", params.tokenInMint, tokenInProgram),
                propAmmBuyFeeVault: this.getConfigurableVaultPda("prop-amm-buy-fee"),
                propAmmBuyFeeTokenInAccount: this.getConfigurableVaultAta("prop-amm-buy-fee", params.tokenInMint, tokenInProgram),
                permissionlessAuthority: this.pdas.permissionlessVaultAuthorityPda,
                permissionlessTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, this.pdas.permissionlessVaultAuthorityPda, true, tokenInProgram),
                permissionlessTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, this.pdas.permissionlessVaultAuthorityPda, true, tokenOutProgram),
                mintAuthority: this.pdas.mintAuthorityPda,
                bufferAccounts: {
                    bufferState: this.pdas.bufferStatePda,
                    reserveVaultOnycAccount: this.getBufferVaultAta(params.tokenOutMint),
                    managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.tokenOutMint),
                    performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.tokenOutMint),
                },
                marketStats: this.pdas.marketStatsPda,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
                instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
                user: params.user,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
                mainOffer: state.mainOffer as PublicKey,
            })
            .instruction();
    }

    async buildOpenSwapSellIx(params: {
        tokenInAmount: string;
        minimumOut: string;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        const state = await this.getState();
        const onycMint = state.onycMint as PublicKey;
        if (!params.tokenInMint.equals(onycMint)) {
            throw new Error("Prop AMM sell swaps must use ONYC as token in");
        }

        const tokenInProgram = params.tokenInProgram ?? TOKEN_PROGRAM_ID;
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;
        const assetMint = params.tokenOutMint;
        const offer = this.getOfferPda(assetMint, onycMint);

        return await this.program.methods
            .openSwapSell(new BN(params.tokenInAmount), new BN(params.minimumOut))
            .accountsPartial({
                offer,
                propAmmPairState: this.getPropAmmPairPda(offer),
                redemptionOffer: this.getRedemptionOfferPda(onycMint, assetMint),
                state: this.statePda,
                offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
                redemptionVaultAuthority: this.pdas.redemptionVaultAuthorityPda,
                redemptionVaultTokenInAccount: getAssociatedTokenAddressSync(onycMint, this.pdas.redemptionVaultAuthorityPda, true, tokenInProgram),
                redemptionVaultTokenOutAccount: getAssociatedTokenAddressSync(assetMint, this.pdas.redemptionVaultAuthorityPda, true, tokenOutProgram),
                tokenInMint: onycMint,
                tokenInProgram,
                tokenOutMint: assetMint,
                tokenOutProgram,
                userTokenInAccount: getAssociatedTokenAddressSync(onycMint, params.user, false, tokenInProgram),
                userTokenOutAccount: getAssociatedTokenAddressSync(assetMint, params.user, false, tokenOutProgram),
                propAmmProceedsVault: this.getConfigurableVaultPda("prop-amm-proceeds"),
                propAmmProceedsTokenInAccount: this.getConfigurableVaultAta("prop-amm-proceeds", onycMint, tokenInProgram),
                propAmmSellFeeVault: this.getConfigurableVaultPda("prop-amm-sell-fee"),
                propAmmSellFeeTokenInAccount: this.getConfigurableVaultAta("prop-amm-sell-fee", onycMint, tokenInProgram),
                mintAuthority: this.pdas.mintAuthorityPda,
                bufferAccounts: {
                    bufferState: this.pdas.bufferStatePda,
                    reserveVaultOnycAccount: this.getBufferVaultAta(onycMint),
                    managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(onycMint),
                    performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(onycMint),
                },
                marketStats: this.pdas.marketStatsPda,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
                instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
                user: params.user,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
                mainOffer: state.mainOffer as PublicKey,
                offerVaultOnycAccount: getAssociatedTokenAddressSync(onycMint, this.pdas.offerVaultAuthorityPda, true, tokenInProgram),
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
        const offer = (state.mainOffer as PublicKey).equals(PublicKey.default) ? PublicKey.default : (state.mainOffer as PublicKey);

        return await this.program.methods
            .mintTo(new BN(params.amount))
            .accountsPartial({
                tokenProgram: TOKEN_PROGRAM_ID,
                mainOffer: offer,
                bufferAccounts: {
                    bufferState: this.pdas.bufferStatePda,
                    reserveVaultOnycAccount: getAssociatedTokenAddressSync(onycMint, this.pdas.reserveVaultAuthorityPda, true, TOKEN_PROGRAM_ID),
                    managementFeeVaultOnycAccount: getAssociatedTokenAddressSync(onycMint, this.pdas.managementFeeVaultPda, true, TOKEN_PROGRAM_ID),
                    performanceFeeVaultOnycAccount: getAssociatedTokenAddressSync(onycMint, this.pdas.performanceFeeVaultPda, true, TOKEN_PROGRAM_ID),
                },
                marketStats: this.pdas.marketStatsPda,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
            })
            .instruction();
    }

    async buildMakeRedemptionOfferIx(params: {
        tokenInMint: PublicKey;
        tokenInProgram: PublicKey;
        tokenOutMint: PublicKey;
        tokenOutProgram: PublicKey;
        feeBasisPoints: number;
        feeBasisPointsPropAmmSell?: number;
        boss: PublicKey;
    }) {
        return await this.program.methods
            .makeRedemptionOffer(params.feeBasisPoints, params.feeBasisPointsPropAmmSell ?? 0)
            .accountsPartial({
                tokenInMint: params.tokenInMint,
                tokenInProgram: params.tokenInProgram,
                tokenOutMint: params.tokenOutMint,
                tokenOutProgram: params.tokenOutProgram,
                boss: params.boss,
            })
            .instruction();
    }

    async buildCreateRedemptionRequestIx(params: { redemptionOfferPda: PublicKey; tokenInMint: PublicKey; amount: number; requestId: string; redeemer: PublicKey; tokenProgram?: PublicKey }) {
        const redemptionOffer = await this.program.account.redemptionOffer.fetch(params.redemptionOfferPda);
        const redemptionRequest = this.getRedemptionRequestPda(params.redemptionOfferPda, params.redeemer, params.requestId);

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
            .createRedemptionRequest(new BN(params.amount), params.requestId)
            .accountsPartial({
                redemptionOffer: params.redemptionOfferPda,
                offer: redemptionOffer.offer,
                redemptionRequest,
                tokenInMint: params.tokenInMint,
                redeemer: params.redeemer,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
            })
            .instruction();
    }

    async buildFulfillRedemptionRequestIx(params: {
        redemptionOfferPda: PublicKey;
        redemptionRequestPda: PublicKey;
        worker: PublicKey;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
        amount: BN;
    }) {
        const tokenInProgram = params.tokenInProgram ?? TOKEN_PROGRAM_ID;
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;
        const redemptionOffer = await this.program.account.redemptionOffer.fetch(params.redemptionOfferPda);
        const redemptionRequest = await this.program.account.redemptionRequest.fetch(params.redemptionRequestPda);
        const redeemer = redemptionRequest.redeemer as PublicKey;
        const onycMint = params.tokenInMint;
        const state = await this.getState();

        return await this.program.methods
            .fulfillRedemptionRequest(params.amount)
            .accountsPartial({
                state: this.statePda,
                offer: redemptionOffer.offer as PublicKey,
                redemptionOffer: params.redemptionOfferPda,
                redemptionRequest: params.redemptionRequestPda,
                redemptionVaultAuthority: this.pdas.redemptionVaultAuthorityPda,
                vaultTokenInAccount: getAssociatedTokenAddressSync(params.tokenInMint, this.pdas.redemptionVaultAuthorityPda, true, tokenInProgram),
                vaultTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, this.pdas.redemptionVaultAuthorityPda, true, tokenOutProgram),
                tokenInMint: params.tokenInMint,
                tokenInProgram,
                tokenOutMint: params.tokenOutMint,
                tokenOutProgram,
                userTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, redeemer, false, tokenOutProgram),
                offerProceedsVault: this.getConfigurableVaultPda("offer-proceeds"),
                offerProceedsTokenInAccount: this.getConfigurableVaultAta("offer-proceeds", params.tokenInMint, tokenInProgram),
                redemptionFeeVault: this.getConfigurableVaultPda("redemption-fee"),
                redemptionFeeTokenInAccount: this.getConfigurableVaultAta("redemption-fee", params.tokenInMint, tokenInProgram),
                mintAuthority: this.pdas.mintAuthorityPda,
                redeemer,
                worker: params.worker,
                bufferAccounts: {
                    bufferState: this.pdas.bufferStatePda,
                    reserveVaultOnycAccount: this.getBufferVaultAta(onycMint),
                    managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(onycMint),
                    performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(onycMint),
                },
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
                offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
                offerVaultOnycAccount: getAssociatedTokenAddressSync(onycMint, this.pdas.offerVaultAuthorityPda, true, tokenInProgram),
                marketStats: this.pdas.marketStatsPda,
                circulatingSupplyExcludedBalance: this.pdas.circulatingSupplyExcludedBalancePda,
                mainOffer: state.mainOffer as PublicKey,
            })
            .instruction();
    }

    async buildCancelRedemptionRequestIx(params: {
        redemptionOfferPda: PublicKey;
        redemptionRequestPda: PublicKey;
        signer: PublicKey;
        tokenInMint: PublicKey;
        tokenProgram?: PublicKey;
    }) {
        const tokenProgram = params.tokenProgram ?? TOKEN_PROGRAM_ID;
        const state = await this.getState();
        const redemptionRequest = await this.program.account.redemptionRequest.fetch(params.redemptionRequestPda);
        const redeemer = redemptionRequest.redeemer as PublicKey;

        return await this.program.methods
            .cancelRedemptionRequest()
            .accountsPartial({
                state: this.statePda,
                redemptionOffer: params.redemptionOfferPda,
                redemptionRequest: params.redemptionRequestPda,
                signer: params.signer,
                redeemer,
                worker: state.worker as PublicKey,
                redemptionVaultAuthority: this.pdas.redemptionVaultAuthorityPda,
                tokenInMint: params.tokenInMint,
                vaultTokenAccount: getAssociatedTokenAddressSync(params.tokenInMint, this.pdas.redemptionVaultAuthorityPda, true, tokenProgram),
                redeemerTokenAccount: getAssociatedTokenAddressSync(params.tokenInMint, redeemer, false, tokenProgram),
                tokenProgram,
                systemProgram: anchor.web3.SystemProgram.programId,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
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

    async buildUpdateRedemptionOfferPropAmmSellFeeIx(params: { redemptionOfferPda: PublicKey; newFeeBasisPointsPropAmmSell: number; boss: PublicKey }) {
        return await this.program.methods
            .updateRedemptionOfferPropAmmSellFee(params.newFeeBasisPointsPropAmmSell)
            .accountsPartial({
                redemptionOffer: params.redemptionOfferPda,
                boss: params.boss,
            })
            .instruction();
    }

    async buildSetRedemptionOfferDisabledIx(params: { redemptionOfferPda: PublicKey; disabled: boolean; signer: PublicKey }) {
        return await this.program.methods
            .setRedemptionOfferDisabled(params.disabled)
            .accountsPartial({
                redemptionOffer: params.redemptionOfferPda,
                signer: params.signer,
            })
            .instruction();
    }

    async buildUpdateRedemptionOfferVaultTargetIx(params: { redemptionOfferPda: PublicKey; vaultTargetBps: number; boss: PublicKey }) {
        return await this.program.methods
            .updateRedemptionOfferVaultTarget(params.vaultTargetBps)
            .accountsPartial({
                redemptionOffer: params.redemptionOfferPda,
                boss: params.boss,
            })
            .instruction();
    }

    async prepareTransactionMultipleIxs(params: { ixs: TransactionInstruction[]; payer: PublicKey }) {
        const tx = new Transaction();
        tx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }));
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
