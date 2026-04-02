import { Keypair, PublicKey, SystemProgram, TransactionInstruction, Transaction } from "@solana/web3.js";
import { AnchorProvider, BN, Program, Wallet } from "@coral-xyz/anchor";
import { Onreapp } from "../target/types/onreapp";
import { BPF_UPGRADEABLE_LOADER_PROGRAM_ID, ONREAPP_PROGRAM_ID, TestHelper } from "./test_helper.ts";
import idl from "../target/idl/onreapp.json";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { createHash } from "crypto";

/**
 * Helper to check view transaction errors and throw with logs
 */
function parseViewError(result: any): void {
    if ("Err" in result || typeof result.err === "function") {
        const logs = result.meta().logs();
        const errorMessage = logs.join("\n");
        throw new Error(errorMessage);
    }
}

function instructionDiscriminator(name: string): Buffer {
    return createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}

export class OnreProgram {
    program: Program<Onreapp>;
    testHelper: TestHelper;

    pdas: {
        statePda: PublicKey;
        offerVaultAuthorityPda: PublicKey;
        redemptionVaultAuthorityPda: PublicKey;
        permissionlessAuthorityPda: PublicKey;
        mintAuthorityPda: PublicKey;
        bufferStatePda: PublicKey;
        reserveVaultAuthorityPda: PublicKey;
        marketStatsPda: PublicKey;
        managementFeeVaultAuthorityPda: PublicKey;
        performanceFeeVaultAuthorityPda: PublicKey;
        redemptionFeeVaultAuthorityPda: PublicKey;
    } = {
        statePda: PublicKey.findProgramAddressSync([Buffer.from("state")], ONREAPP_PROGRAM_ID)[0],
        offerVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("offer_vault_authority")], ONREAPP_PROGRAM_ID)[0],
        redemptionVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("redemption_offer_vault_authority")], ONREAPP_PROGRAM_ID)[0],
        permissionlessAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], ONREAPP_PROGRAM_ID)[0],
        mintAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("mint_authority")], ONREAPP_PROGRAM_ID)[0],
        bufferStatePda: PublicKey.findProgramAddressSync([Buffer.from("buffer_state")], ONREAPP_PROGRAM_ID)[0],
        reserveVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("reserve_vault_authority")], ONREAPP_PROGRAM_ID)[0],
        marketStatsPda: PublicKey.findProgramAddressSync([Buffer.from("market_stats")], ONREAPP_PROGRAM_ID)[0],
        managementFeeVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("management_fee_vault_authority")], ONREAPP_PROGRAM_ID)[0],
        performanceFeeVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("performance_fee_vault_authority")], ONREAPP_PROGRAM_ID)[0],
        redemptionFeeVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("redemption_fee_vault_authority")], ONREAPP_PROGRAM_ID)[0],
    };

    constructor(testHelper: TestHelper) {
        this.testHelper = testHelper;

        const wallet = new Wallet(testHelper.payer);
        const provider = new AnchorProvider(testHelper.getConnection() as any, wallet, { commitment: "processed" });

        this.program = new Program<Onreapp>(idl as Onreapp, provider);
    }

    private async rpcWithOptionalSigner(tx: any, signer?: Keypair) {
        if (signer) {
            tx.signers([signer]);
        }
        await tx.rpc();
    }

    // Instructions
    async initialize(params: { onycMint: PublicKey }) {
        await this.program.methods
            .initialize()
            .accounts({
                boss: this.testHelper.payer.publicKey,
                onycMint: params.onycMint,
                programData: PublicKey.findProgramAddressSync([this.program.programId.toBuffer()], BPF_UPGRADEABLE_LOADER_PROGRAM_ID)[0],
            })
            .rpc();
    }

    async makeOffer(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        feeBasisPoints?: number;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
        withApproval?: boolean;
        allowPermissionless?: boolean;
    }) {
        const feeBasisPoints = params.feeBasisPoints ?? 0;
        const tx = this.program.methods.makeOffer(feeBasisPoints, params.withApproval ?? false, params.allowPermissionless ?? false).accounts({
            tokenInMint: params.tokenInMint,
            tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
            tokenOutMint: params.tokenOutMint,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async addOfferVector(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        startTime?: number;
        baseTime: number;
        basePrice: number;
        apr: number;
        priceFixDuration: number;
        signer?: Keypair;
    }) {
        const tx = this.program.methods
            .addOfferVector(
                params.startTime == null ? null : new BN(params.startTime),
                new BN(params.baseTime),
                new BN(params.basePrice),
                new BN(params.apr),
                new BN(params.priceFixDuration),
            )
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
            });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async updateOfferFee(params: { tokenInMint: PublicKey; tokenOutMint: PublicKey; newFee: number; signer?: Keypair }) {
        const tx = this.program.methods.updateOfferFee(params.newFee).accounts({
            tokenInMint: params.tokenInMint,
            tokenOutMint: params.tokenOutMint,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async deleteOfferVector(tokenInMint: PublicKey, tokenOutMint: PublicKey, vectorStartTime: number, signer?: Keypair) {
        const tx = this.program.methods.deleteOfferVector(new BN(vectorStartTime)).accounts({
            tokenInMint: tokenInMint,
            tokenOutMint: tokenOutMint,
        });

        await this.rpcWithOptionalSigner(tx, signer);
    }

    async deleteAllOfferVectors(tokenInMint: PublicKey, tokenOutMint: PublicKey, signer?: Keypair) {
        const tx = this.program.methods.deleteAllOfferVectors().accounts({
            tokenInMint: tokenInMint,
            tokenOutMint: tokenOutMint,
        });

        await this.rpcWithOptionalSigner(tx, signer);
    }

    async takeOffer(params: {
        tokenInAmount: number;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        const tx = this.program.methods.takeOffer(new BN(params.tokenInAmount), null).accounts({
            tokenInMint: params.tokenInMint,
            tokenOutMint: params.tokenOutMint,
            user: params.user,
            tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
            tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async takeOfferV2(params: {
        tokenInAmount: number;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        const tx = this.program.methods.takeOfferV2(new BN(params.tokenInAmount), null).accountsPartial({
            tokenInMint: params.tokenInMint,
            tokenOutMint: params.tokenOutMint,
            user: params.user,
            tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
            tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
            marketStats: this.pdas.marketStatsPda,
            bufferAccounts: {
                bufferState: this.pdas.bufferStatePda,
                reserveVaultOnycAccount: this.getBufferVaultAta(params.tokenOutMint),
                managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.tokenOutMint),
                performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.tokenOutMint),
            },
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async takeOfferPermissionless(params: {
        tokenInAmount: number;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        const tx = this.program.methods.takeOfferPermissionless(new BN(params.tokenInAmount), null).accountsPartial({
            tokenInMint: params.tokenInMint,
            tokenOutMint: params.tokenOutMint,
            user: params.user,
            tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
            tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
            boss: this.testHelper.payer.publicKey,
            vaultAuthority: this.pdas.offerVaultAuthorityPda,
            permissionlessAuthority: this.pdas.permissionlessAuthorityPda,
            mintAuthority: this.pdas.mintAuthorityPda,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async takeOfferPermissionlessV2(params: {
        tokenInAmount: number;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        const tx = this.program.methods.takeOfferPermissionlessV2(new BN(params.tokenInAmount), null).accountsPartial({
            tokenInMint: params.tokenInMint,
            tokenOutMint: params.tokenOutMint,
            user: params.user,
            tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
            tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
            boss: this.testHelper.payer.publicKey,
            vaultAuthority: this.pdas.offerVaultAuthorityPda,
            permissionlessAuthority: this.pdas.permissionlessAuthorityPda,
            mintAuthority: this.pdas.mintAuthorityPda,
            marketStats: this.pdas.marketStatsPda,
            bufferAccounts: {
                bufferState: this.pdas.bufferStatePda,
                reserveVaultOnycAccount: this.getBufferVaultAta(params.tokenOutMint),
                managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.tokenOutMint),
                performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.tokenOutMint),
            },
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async offerVaultDeposit(params: { amount: number; tokenMint: PublicKey; signer?: Keypair; tokenProgram?: PublicKey }) {
        const depositor = params.signer?.publicKey ?? this.testHelper.payer.publicKey;
        const tx = this.program.methods.offerVaultDeposit(new BN(params.amount)).accounts({
                depositor,
            tokenMint: params.tokenMint,
            tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async offerVaultWithdraw(params: { amount: number; tokenMint: PublicKey; signer?: Keypair; tokenProgram?: PublicKey }) {
        const tx = this.program.methods.offerVaultWithdraw(new BN(params.amount)).accounts({
            tokenMint: params.tokenMint,
            tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async redemptionVaultDeposit(params: { amount: number; tokenMint: PublicKey; signer?: Keypair; tokenProgram?: PublicKey }) {
        const depositor = params.signer?.publicKey ?? this.testHelper.payer.publicKey;
        const tx = this.program.methods.redemptionVaultDeposit(new BN(params.amount)).accounts({
                depositor,
            tokenMint: params.tokenMint,
            tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async redemptionVaultWithdraw(params: { amount: number; tokenMint: PublicKey; signer?: Keypair; tokenProgram?: PublicKey }) {
        const tx = this.program.methods.redemptionVaultWithdraw(new BN(params.amount)).accounts({
            tokenMint: params.tokenMint,
            tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async initializePermissionlessAuthority(params: { accountName: string }) {
        await this.program.methods.initializePermissionlessAuthority(params.accountName).rpc();
    }

    async transferMintAuthorityToProgram(params: { mint: PublicKey; signer?: Keypair; tokenProgram?: PublicKey }) {
        const tx = this.program.methods.transferMintAuthorityToProgram().accounts({
            mint: params.mint,
            tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async transferMintAuthorityToBoss(params: { mint: PublicKey; signer?: Keypair; tokenProgram?: PublicKey }) {
        const tx = this.program.methods.transferMintAuthorityToBoss().accounts({
            mint: params.mint,
            tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async addAdmin(params: { admin: PublicKey; signer?: Keypair }) {
        const tx = this.program.methods.addAdmin(params.admin);

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async removeAdmin(params: { admin: PublicKey; signer?: Keypair }) {
        const tx = this.program.methods.removeAdmin(params.admin);

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async proposeBoss(params: { newBoss: PublicKey; signer?: Keypair }) {
        const tx = this.program.methods.proposeBoss(params.newBoss).accounts({
            boss: params.signer ? params.signer.publicKey : this.testHelper.payer.publicKey,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async acceptBoss(params: { newBoss: Keypair }) {
        const tx = this.program.methods
            .acceptBoss()
            .accounts({
                newBoss: params.newBoss.publicKey,
            })
            .signers([params.newBoss]);

        await tx.rpc();
    }

    async setKillSwitch(params: { enable: boolean; signer?: Keypair }) {
        const tx = this.program.methods.setKillSwitch(params.enable).accounts({
            signer: params.signer ? params.signer.publicKey : this.testHelper.payer.publicKey,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async setOnycMint(params: { onycMint: PublicKey; signer?: Keypair }) {
        const tx = this.program.methods.setOnycMint().accounts({
            onycMint: params.onycMint,
        });

        await this.rpcWithOptionalSigner(tx, params?.signer);
    }

    async setRedemptionAdmin(params: { redemptionAdmin: PublicKey; signer?: Keypair }) {
        const tx = this.program.methods.setRedemptionAdmin(params.redemptionAdmin).accounts({});

        await this.rpcWithOptionalSigner(tx, params?.signer);
    }

    async initializeBuffer(params: { offer: PublicKey; onycMint: PublicKey; signer?: Keypair }) {
        const tx = this.program.methods.initializeBuffer().accountsPartial({
            offer: params.offer,
            onycMint: params.onycMint,
            bufferState: this.pdas.bufferStatePda,
            reserveVaultAuthority: this.pdas.reserveVaultAuthorityPda,
            reserveVaultOnycAccount: this.getBufferVaultAta(params.onycMint),
            managementFeeVaultAuthority: this.pdas.managementFeeVaultAuthorityPda,
            managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.onycMint),
            performanceFeeVaultAuthority: this.pdas.performanceFeeVaultAuthorityPda,
            performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.onycMint),
            tokenProgram: TOKEN_PROGRAM_ID,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async setMainOffer(params: { offer: PublicKey; signer?: Keypair }) {
        const signer = params.signer ?? this.testHelper.payer;
        const instruction = new TransactionInstruction({
            programId: this.program.programId,
            keys: [
                { pubkey: this.pdas.statePda, isSigner: false, isWritable: true },
                { pubkey: signer.publicKey, isSigner: true, isWritable: false },
                { pubkey: params.offer, isSigner: false, isWritable: false },
            ],
            data: instructionDiscriminator("set_main_offer"),
        });

        const tx = new Transaction().add(instruction);
        await this.testHelper.sendAndConfirmTransaction(tx, [signer]);
    }

    async setBufferGrossYield(params: { grossYield: number; signer?: Keypair }) {
        const tx = this.program.methods.setBufferGrossApr(new BN(params.grossYield)).accountsPartial({});

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async setBufferFeeConfig(params: { managementFeeBasisPoints: number; performanceFeeBasisPoints: number; signer?: Keypair }) {
        const feeRecipient = (params.signer ?? this.testHelper.payer).publicKey;
        const tx = this.program.methods.setBufferFeeConfig(
            params.managementFeeBasisPoints,
            feeRecipient,
            params.performanceFeeBasisPoints,
            feeRecipient,
        );

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async burnForNavIncrease(params: { tokenInMint: PublicKey; onycMint: PublicKey; assetAdjustmentAmount: number; targetNav: number; signer?: Keypair }) {
        const signer = params.signer ?? this.testHelper.payer;
        const offerPda = PublicKey.findProgramAddressSync([Buffer.from("offer"), params.tokenInMint.toBuffer(), params.onycMint.toBuffer()], ONREAPP_PROGRAM_ID)[0];
        await this.testHelper.advanceSlot();
        const tx = await this.program.methods
            .burnForNavIncrease(new BN(params.assetAdjustmentAmount), new BN(params.targetNav))
            .accountsPartial({
                boss: signer.publicKey,
                offer: offerPda,
                tokenInMint: params.tokenInMint,
                onycMint: params.onycMint,
                bufferState: this.pdas.bufferStatePda,
                offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
                vaultTokenOutAccount: getAssociatedTokenAddressSync(params.onycMint, this.pdas.offerVaultAuthorityPda, true, TOKEN_PROGRAM_ID),
                reserveVaultAuthority: this.pdas.reserveVaultAuthorityPda,
                reserveVaultOnycAccount: this.getBufferVaultAta(params.onycMint),
                managementFeeVaultAuthority: this.pdas.managementFeeVaultAuthorityPda,
                managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.onycMint),
                performanceFeeVaultAuthority: this.pdas.performanceFeeVaultAuthorityPda,
                performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.onycMint),
                mintAuthority: this.pdas.mintAuthorityPda,
                marketStats: this.pdas.marketStatsPda,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
            })
            .transaction();
        await this.testHelper.sendAndConfirmTransaction(tx, [signer]);
    }

    async withdrawManagementFees(params: { onycMint: PublicKey; amount: number; signer?: Keypair }) {
        const signer = params.signer ?? this.testHelper.payer;
        const data = Buffer.concat([
            instructionDiscriminator("withdraw_management_fees"),
            new BN(params.amount).toArrayLike(Buffer, "le", 8),
        ]);
        const instruction = new TransactionInstruction({
            programId: this.program.programId,
            keys: [
                { pubkey: this.pdas.statePda, isSigner: false, isWritable: false },
                { pubkey: this.pdas.bufferStatePda, isSigner: false, isWritable: true },
                { pubkey: this.pdas.managementFeeVaultAuthorityPda, isSigner: false, isWritable: false },
                { pubkey: signer.publicKey, isSigner: false, isWritable: false },
                { pubkey: params.onycMint, isSigner: false, isWritable: true },
                { pubkey: getAssociatedTokenAddressSync(params.onycMint, signer.publicKey, false, TOKEN_PROGRAM_ID), isSigner: false, isWritable: true },
                { pubkey: this.getManagementFeeVaultAta(params.onycMint), isSigner: false, isWritable: true },
                { pubkey: signer.publicKey, isSigner: true, isWritable: true },
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
                { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            ],
            data,
        });

        const tx = new Transaction().add(instruction);
        await this.testHelper.sendAndConfirmTransaction(tx, [signer]);
    }

    async withdrawPerformanceFees(params: { onycMint: PublicKey; amount: number; signer?: Keypair }) {
        const signer = params.signer ?? this.testHelper.payer;
        const data = Buffer.concat([
            instructionDiscriminator("withdraw_performance_fees"),
            new BN(params.amount).toArrayLike(Buffer, "le", 8),
        ]);
        const instruction = new TransactionInstruction({
            programId: this.program.programId,
            keys: [
                { pubkey: this.pdas.statePda, isSigner: false, isWritable: false },
                { pubkey: this.pdas.bufferStatePda, isSigner: false, isWritable: true },
                { pubkey: this.pdas.performanceFeeVaultAuthorityPda, isSigner: false, isWritable: false },
                { pubkey: signer.publicKey, isSigner: false, isWritable: false },
                { pubkey: params.onycMint, isSigner: false, isWritable: true },
                { pubkey: getAssociatedTokenAddressSync(params.onycMint, signer.publicKey, false, TOKEN_PROGRAM_ID), isSigner: false, isWritable: true },
                { pubkey: this.getPerformanceFeeVaultAta(params.onycMint), isSigner: false, isWritable: true },
                { pubkey: signer.publicKey, isSigner: true, isWritable: true },
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
                { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            ],
            data,
        });

        const tx = new Transaction().add(instruction);
        await this.testHelper.sendAndConfirmTransaction(tx, [signer]);
    }

    async makeRedemptionOffer(params: { offer: PublicKey; feeBasisPoints?: number; signer?: Keypair; tokenInProgram?: PublicKey; tokenOutProgram?: PublicKey }) {
        // Fetch the offer to get token mints
        const offer = await this.program.account.offer.fetch(params.offer);

        const tx = this.program.methods.makeRedemptionOffer(params.feeBasisPoints ?? 0).accounts({
            tokenInMint: offer.tokenOutMint,
            tokenOutMint: offer.tokenInMint,
            tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
            tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
            signer: params.signer ? params.signer.publicKey : this.testHelper.payer.publicKey,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async updateRedemptionOfferFee(params: { redemptionOffer: PublicKey; newFeeBasisPoints: number; signer?: Keypair }) {
        const tx = this.program.methods.updateRedemptionOfferFee(params.newFeeBasisPoints).accountsPartial({
            redemptionOffer: params.redemptionOffer,
            boss: params.signer ? params.signer.publicKey : this.testHelper.payer.publicKey,
        });

        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async mintTo(params: { amount: number; signer?: Keypair }) {
        const state = await this.getState();
        const onycMint = state.onycMint as PublicKey;
        const offer = (state.mainOffer as PublicKey).equals(SystemProgram.programId)
            ? SystemProgram.programId
            : (state.mainOffer as PublicKey);

        const tx = this.program.methods.mintTo(new BN(params.amount)).accountsPartial({
            tokenProgram: TOKEN_PROGRAM_ID,
            offer,
            bufferAccounts: {
                bufferState: this.pdas.bufferStatePda,
                reserveVaultOnycAccount: this.getBufferVaultAta(onycMint),
                managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(onycMint),
                performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(onycMint),
            },
            offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
            offerVaultOnycAccount: getAssociatedTokenAddressSync(onycMint, this.pdas.offerVaultAuthorityPda, true, TOKEN_PROGRAM_ID),
            marketStats: this.pdas.marketStatsPda,
        });

        await this.rpcWithOptionalSigner(tx, params?.signer);
    }

    async configureMaxSupply(params: { maxSupply: number; signer?: Keypair }) {
        const tx = this.program.methods.configureMaxSupply(new BN(params.maxSupply));

        await this.rpcWithOptionalSigner(tx, params?.signer);
    }

    async closeState(params?: { signer?: Keypair }) {
        const tx = this.program.methods.closeState().accounts({
            state: this.pdas.statePda,
        });

        await this.rpcWithOptionalSigner(tx, params?.signer);
    }

    async getNAV(params: { tokenInMint: PublicKey; tokenOutMint: PublicKey }): Promise<number> {
        const tx = await this.program.methods
            .getNav()
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
            })
            .transaction();

        tx.recentBlockhash = this.testHelper.svm.latestBlockhash();
        tx.feePayer = this.testHelper.payer.publicKey;
        tx.sign(this.testHelper.payer);

        const result = this.testHelper.svm.simulateTransaction(tx);

        // Check for errors
        parseViewError(result);

        const meta = result.meta();
        const returnData = meta.returnData();

        if (!returnData || returnData.data().length === 0) {
            throw new Error(`No return data from getNAV`);
        }

        // Parse the return data as u64 (8 bytes, little-endian)
        const data = returnData.data();
        const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
        const nav = Number(view.getBigUint64(0, true));

        return nav;
    }

    async getAPY(params: { tokenInMint: PublicKey; tokenOutMint: PublicKey }): Promise<number> {
        const tx = await this.program.methods
            .getApy()
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
            })
            .transaction();

        tx.recentBlockhash = this.testHelper.svm.latestBlockhash();
        tx.feePayer = this.testHelper.payer.publicKey;
        tx.sign(this.testHelper.payer);

        const result = this.testHelper.svm.simulateTransaction(tx);

        // Check for errors
        parseViewError(result);

        const meta = result.meta();
        const returnData = meta.returnData();

        if (!returnData || returnData.data().length === 0) {
            throw new Error(`No return data from getAPY`);
        }

        // Parse the return data as u64 (8 bytes, little-endian)
        const data = returnData.data();
        const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
        const apy = Number(view.getBigUint64(0, true));

        return apy;
    }

    async getNavAdjustment(params: { tokenInMint: PublicKey; tokenOutMint: PublicKey }): Promise<number> {
        const tx = await this.program.methods
            .getNavAdjustment()
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
            })
            .transaction();

        tx.recentBlockhash = this.testHelper.svm.latestBlockhash();
        tx.feePayer = this.testHelper.payer.publicKey;
        tx.sign(this.testHelper.payer);

        const result = this.testHelper.svm.simulateTransaction(tx);

        // Check for errors
        parseViewError(result);

        const meta = result.meta();
        const returnData = meta.returnData();

        if (!returnData || returnData.data().length === 0) {
            throw new Error(`No return data from getNavAdjustment`);
        }

        // Parse the return data as i64 (8 bytes, little-endian, signed)
        const data = returnData.data();
        const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
        const adjustment = Number(view.getBigInt64(0, true));

        return adjustment;
    }

    async getTVL(params: { tokenInMint: PublicKey; tokenOutMint: PublicKey; tokenOutProgram?: PublicKey }): Promise<BN> {
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;
        const tx = await this.program.methods
            .getTvl()
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                tokenOutProgram: tokenOutProgram,
                vaultTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, this.pdas.offerVaultAuthorityPda, true, tokenOutProgram),
            })
            .transaction();

        tx.recentBlockhash = this.testHelper.svm.latestBlockhash();
        tx.feePayer = this.testHelper.payer.publicKey;
        tx.sign(this.testHelper.payer);

        const result = this.testHelper.svm.simulateTransaction(tx);

        // Check for errors
        parseViewError(result);

        const meta = result.meta();
        const returnData = meta.returnData();

        if (!returnData || returnData.data().length === 0) {
            throw new Error(`No return data from getTVL`);
        }

        // Parse the return data as u64 (8 bytes, little-endian)
        const data = returnData.data();
        const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
        const tvl = view.getBigUint64(0, true);

        return new BN(tvl.toString());
    }

    async getCirculatingSupply(params: { onycMint: PublicKey; tokenOutProgram?: PublicKey }): Promise<BN> {
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;

        const tx = await this.program.methods
            .getCirculatingSupply()
            .accounts({
                tokenProgram: tokenOutProgram,
                onycVaultAccount: getAssociatedTokenAddressSync(params.onycMint, this.pdas.offerVaultAuthorityPda, true, tokenOutProgram),
            })
            .transaction();

        tx.recentBlockhash = this.testHelper.svm.latestBlockhash();
        tx.feePayer = this.testHelper.payer.publicKey;
        tx.sign(this.testHelper.payer);

        const result = this.testHelper.svm.simulateTransaction(tx);

        // Check for errors
        parseViewError(result);

        const meta = result.meta();
        const returnData = meta.returnData();

        if (!returnData || returnData.data().length === 0) {
            throw new Error(`No return data from getCirculatingSupply`);
        }

        // Parse the return data as u64 (8 bytes, little-endian)
        const data = returnData.data();
        const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
        const circulatingSupply = view.getBigUint64(0, true);

        return new BN(circulatingSupply.toString());
    }

    // Accounts
    async getOffer(tokenInMint: PublicKey, tokenOutMint: PublicKey) {
        return await this.program.account.offer.fetch(this.getOfferPda(tokenInMint, tokenOutMint));
    }

    getOfferPda(tokenInMint: PublicKey, tokenOutMint: PublicKey) {
        return PublicKey.findProgramAddressSync([Buffer.from("offer"), tokenInMint.toBuffer(), tokenOutMint.toBuffer()], this.program.programId)[0];
    }

    async getRedemptionOffer(tokenInMint: PublicKey, tokenOutMint: PublicKey) {
        return await this.program.account.redemptionOffer.fetch(this.getRedemptionOfferPda(tokenInMint, tokenOutMint));
    }

    getRedemptionOfferPda(tokenInMint: PublicKey, tokenOutMint: PublicKey) {
        return PublicKey.findProgramAddressSync([Buffer.from("redemption_offer"), tokenInMint.toBuffer(), tokenOutMint.toBuffer()], this.program.programId)[0];
    }

    async getState() {
        return await this.program.account.state.fetch(this.pdas.statePda);
    }

    async getBufferState() {
        return await this.program.account.bufferState.fetch(this.pdas.bufferStatePda);
    }

    getBufferVaultAta(onycMint: PublicKey) {
        return getAssociatedTokenAddressSync(onycMint, this.pdas.reserveVaultAuthorityPda, true, TOKEN_PROGRAM_ID);
    }

    getManagementFeeVaultAta(onycMint: PublicKey) {
        return getAssociatedTokenAddressSync(onycMint, this.pdas.managementFeeVaultAuthorityPda, true, TOKEN_PROGRAM_ID);
    }

    getPerformanceFeeVaultAta(onycMint: PublicKey) {
        return getAssociatedTokenAddressSync(onycMint, this.pdas.performanceFeeVaultAuthorityPda, true, TOKEN_PROGRAM_ID);
    }

    async getPermissionlessAuthority() {
        return await this.program.account.permissionlessAuthority.fetch(this.pdas.permissionlessAuthorityPda);
    }

    getMarketStatsPda(): PublicKey {
        return this.pdas.marketStatsPda;
    }

    async getMarketStats() {
        return await this.program.account.marketStats.fetch(this.pdas.marketStatsPda);
    }

    async addApprover(params: { trusted: PublicKey; signer?: Keypair }) {
        const tx = this.program.methods.addApprover(params.trusted);
        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async removeApprover(params: { approver: PublicKey; signer?: Keypair }) {
        const tx = this.program.methods.removeApprover(params.approver);
        await this.rpcWithOptionalSigner(tx, params.signer);
    }

    async createRedemptionRequest(params: { redemptionOffer: PublicKey; redeemer: Keypair; amount: number; tokenProgram?: PublicKey }) {
        const redemptionOffer = await this.program.account.redemptionOffer.fetch(params.redemptionOffer);
        const tokenProgram = params.tokenProgram ?? TOKEN_PROGRAM_ID;

        const redeemerTokenAccount = getAssociatedTokenAddressSync(redemptionOffer.tokenInMint, params.redeemer.publicKey, false, tokenProgram);

        const vaultTokenAccount = getAssociatedTokenAddressSync(redemptionOffer.tokenInMint, this.pdas.redemptionVaultAuthorityPda, true, tokenProgram);

        const tx = this.program.methods
            .createRedemptionRequest(new BN(params.amount))
            .accountsPartial({
                redemptionOffer: params.redemptionOffer,
                redeemer: params.redeemer.publicKey,
                tokenInMint: redemptionOffer.tokenInMint,
                tokenProgram,
            })
            .signers([params.redeemer]);

        await tx.rpc();
    }

    async cancelRedemptionRequest(params: { redemptionOffer: PublicKey; redemptionRequest: PublicKey; signer: Keypair; redemptionAdmin: PublicKey; tokenProgram?: PublicKey }) {
        const redemptionOffer = await this.program.account.redemptionOffer.fetch(params.redemptionOffer);
        const redemptionRequest = await this.program.account.redemptionRequest.fetch(params.redemptionRequest);
        const tokenProgram = params.tokenProgram ?? TOKEN_PROGRAM_ID;

        const tx = this.program.methods
            .cancelRedemptionRequest()
            .accountsPartial({
                redemptionOffer: params.redemptionOffer,
                redemptionRequest: params.redemptionRequest,
                signer: params.signer.publicKey,
                tokenInMint: redemptionOffer.tokenInMint,
                redeemer: redemptionRequest.redeemer,
                redemptionAdmin: params.redemptionAdmin,
                tokenProgram,
            })
            .signers([params.signer]);

        await tx.rpc();
    }

    async fulfillRedemptionRequest(params: {
        offer: PublicKey;
        redemptionOffer: PublicKey;
        redemptionRequest: PublicKey;
        redeemer: PublicKey;
        redemptionAdmin: Keypair;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        /** Amount of token_in to fulfill. Omit to fulfill the full remaining unfulfilled balance. */
        amount?: BN;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
        /** Fee destination owner. Defaults to redemptionFeeVaultAuthorityPda. */
        feeDestination?: PublicKey;
    }) {
        let amount = params.amount;
        if (amount === undefined) {
            const request = await this.program.account.redemptionRequest.fetch(params.redemptionRequest);
            amount = (request.amount as BN).sub(request.fulfilledAmount as BN);
        }

        const tokenInProgram = params.tokenInProgram ?? TOKEN_PROGRAM_ID;
        const feeDestination = params.feeDestination ?? this.pdas.redemptionFeeVaultAuthorityPda;
        const feeDestinationTokenInAccount = getAssociatedTokenAddressSync(
            params.tokenInMint,
            feeDestination,
            true,
            tokenInProgram
        );

        const tx = this.program.methods
            .fulfillRedemptionRequest(amount)
            .accountsPartial({
                offer: params.offer,
                redemptionOffer: params.redemptionOffer,
                redemptionRequest: params.redemptionRequest,
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
                redeemer: params.redeemer,
                redemptionAdmin: params.redemptionAdmin.publicKey,
                offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
                offerVaultOnycAccount: getAssociatedTokenAddressSync(
                    params.tokenInMint,
                    this.pdas.offerVaultAuthorityPda,
                    true,
                    params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                ),
                marketStats: this.pdas.marketStatsPda,
            })
            .signers([params.redemptionAdmin]);

        await tx.rpc();
    }

    async fulfillRedemptionRequestV2(params: {
        offer: PublicKey;
        redemptionOffer: PublicKey;
        redemptionRequest: PublicKey;
        redeemer: PublicKey;
        redemptionAdmin: Keypair;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        amount?: BN;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        let amount = params.amount;
        if (amount === undefined) {
            const request = await this.program.account.redemptionRequest.fetch(params.redemptionRequest);
            amount = (request.amount as BN).sub(request.fulfilledAmount as BN);
        }

        const tx = this.program.methods
            .fulfillRedemptionRequestV2(amount)
            .accountsPartial({
                offer: params.offer,
                redemptionOffer: params.redemptionOffer,
                redemptionRequest: params.redemptionRequest,
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                tokenInProgram,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
                redeemer: params.redeemer,
                redemptionAdmin: params.redemptionAdmin.publicKey,
                offerVaultAuthority: this.pdas.offerVaultAuthorityPda,
                offerVaultOnycAccount: getAssociatedTokenAddressSync(
                    params.tokenInMint,
                    this.pdas.offerVaultAuthorityPda,
                    true,
                    params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                ),
                marketStats: this.pdas.marketStatsPda,
                bufferAccounts: {
                    bufferState: this.pdas.bufferStatePda,
                    reserveVaultOnycAccount: this.getBufferVaultAta(params.tokenInMint),
                    managementFeeVaultOnycAccount: this.getManagementFeeVaultAta(params.tokenInMint),
                    performanceFeeVaultOnycAccount: this.getPerformanceFeeVaultAta(params.tokenInMint),
                },
                feeDestination,
                feeDestinationTokenInAccount,
            })
            .signers([params.redemptionAdmin]);

        await tx.rpc();
    }

    async setRedemptionFeeDestination(params: {
        feeDestination: PublicKey;
        boss?: Keypair;
    }) {
        const boss = params.boss ?? this.testHelper.payer;
        await this.program.methods
            .setRedemptionFeeDestination(params.feeDestination)
            .accounts({
                boss: boss.publicKey,
            })
            .signers([boss])
            .rpc();
    }

    async getRedemptionRequest(redemptionOffer: PublicKey, counter: number) {
        const pda = this.getRedemptionRequestPda(redemptionOffer, counter);
        return await this.program.account.redemptionRequest.fetch(pda);
    }

    getRedemptionRequestPda(redemptionOffer: PublicKey, counter: number) {
        return PublicKey.findProgramAddressSync(
            [Buffer.from("redemption_request"), redemptionOffer.toBuffer(), new BN(counter).toArrayLike(Buffer, "le", 8)],
            this.program.programId,
        )[0];
    }
}
