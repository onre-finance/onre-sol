import { Keypair, PublicKey } from "@solana/web3.js";
import { AnchorProvider, BN, Program, Wallet } from "@coral-xyz/anchor";
import { Onreapp } from "../target/types/onreapp";
import { BPF_LOADER_PROGRAM_ID, BPF_UPGRADEABLE_LOADER_PROGRAM_ID, ONREAPP_PROGRAM_ID, TestHelper } from "./test_helper.ts";
import idl from "../target/idl/onreapp.json";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";

export { BPF_LOADER_PROGRAM_ID };

/**
 * Helper to check view transaction errors and throw with logs
 */
function parseViewError(result: any): void {
    if ("Err" in result || typeof result.err === 'function') {
        const logs = result.meta().logs();
        const errorMessage = logs.join('\n');
        throw new Error(errorMessage);
    }
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
    } = {
        statePda: PublicKey.findProgramAddressSync([Buffer.from("state")], ONREAPP_PROGRAM_ID)[0],
        offerVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("offer_vault_authority")], ONREAPP_PROGRAM_ID)[0],
        redemptionVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("redemption_offer_vault_authority")], ONREAPP_PROGRAM_ID)[0],
        permissionlessAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], ONREAPP_PROGRAM_ID)[0],
        mintAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("mint_authority")], ONREAPP_PROGRAM_ID)[0]
    };

    constructor(testHelper: TestHelper) {
        this.testHelper = testHelper;

        const wallet = new Wallet(testHelper.payer);
        const provider = new AnchorProvider(
            testHelper.getConnection() as any,
            wallet,
            { commitment: "processed" }
        );

        this.program = new Program<Onreapp>(
            idl as Onreapp,
            provider
        );
    }

    // Instructions
    async initialize(params: { onycMint: PublicKey }) {
        await this.program.methods
            .initialize()
            .accounts({
                boss: this.testHelper.payer.publicKey,
                onycMint: params.onycMint,
                programData: PublicKey.findProgramAddressSync(
                    [this.program.programId.toBuffer()],
                    BPF_UPGRADEABLE_LOADER_PROGRAM_ID
                )[0]
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
        const tx = this.program.methods
            .makeOffer(feeBasisPoints, params.withApproval ?? false, params.allowPermissionless ?? false)
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutMint: params.tokenOutMint
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async addOfferVector(params: {
        tokenInMint: PublicKey,
        tokenOutMint: PublicKey,
        startTime?: number,
        baseTime: number,
        basePrice: number,
        apr: number,
        priceFixDuration: number,
        signer?: Keypair;
    }) {
        const tx = this.program.methods
            .addOfferVector(
                params.startTime == null ? null : new BN(params.startTime),
                new BN(params.baseTime),
                new BN(params.basePrice),
                new BN(params.apr),
                new BN(params.priceFixDuration)
            )
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async updateOfferFee(params: {
        tokenInMint: PublicKey,
        tokenOutMint: PublicKey,
        newFee: number,
        signer?: Keypair
    }) {
        const tx = this.program.methods
            .updateOfferFee(params.newFee)
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async deleteOfferVector(
        tokenInMint: PublicKey,
        tokenOutMint: PublicKey,
        vectorStartTime: number,
        signer?: Keypair
    ) {
        const tx = this.program.methods
            .deleteOfferVector(new BN(vectorStartTime))
            .accounts({
                tokenInMint: tokenInMint,
                tokenOutMint: tokenOutMint
            });

        if (signer) {
            tx.signers([signer]);
        }

        await tx.rpc();
    }

    async deleteAllOfferVectors(
        tokenInMint: PublicKey,
        tokenOutMint: PublicKey,
        signer?: Keypair
    ) {
        const tx = this.program.methods
            .deleteAllOfferVectors()
            .accounts({
                tokenInMint: tokenInMint,
                tokenOutMint: tokenOutMint
            });

        if (signer) {
            tx.signers([signer]);
        }

        await tx.rpc();
    }

    async takeOffer(params: {
        tokenInAmount: number,
        tokenInMint: PublicKey,
        tokenOutMint: PublicKey,
        user: PublicKey,
        signer?: Keypair,
        tokenInProgram?: PublicKey,
        tokenOutProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .takeOffer(new BN(params.tokenInAmount), null)
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async takeOfferPermissionless(params: {
        tokenInAmount: number,
        tokenInMint: PublicKey,
        tokenOutMint: PublicKey,
        user: PublicKey,
        signer?: Keypair,
        tokenInProgram?: PublicKey,
        tokenOutProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .takeOfferPermissionless(new BN(params.tokenInAmount), null)
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
                boss: this.testHelper.payer.publicKey,
                vaultAuthority: this.pdas.offerVaultAuthorityPda,
                permissionlessAuthority: this.pdas.permissionlessAuthorityPda,
                mintAuthority: this.pdas.mintAuthorityPda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async offerVaultDeposit(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .offerVaultDeposit(new BN(params.amount))
            .accounts({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async offerVaultWithdraw(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .offerVaultWithdraw(new BN(params.amount))
            .accounts({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async redemptionVaultDeposit(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .redemptionVaultDeposit(new BN(params.amount))
            .accounts({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async redemptionVaultWithdraw(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .redemptionVaultWithdraw(new BN(params.amount))
            .accounts({
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async initializePermissionlessAuthority(params: { accountName: string }) {
        await this.program.methods
            .initializePermissionlessAuthority(params.accountName)
            .rpc();
    }

    async transferMintAuthorityToProgram(params: { mint: PublicKey, signer?: Keypair, tokenProgram?: PublicKey }) {
        const tx = this.program.methods
            .transferMintAuthorityToProgram()
            .accounts({
                mint: params.mint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async transferMintAuthorityToBoss(params: { mint: PublicKey, signer?: Keypair, tokenProgram?: PublicKey }) {
        const tx = this.program.methods
            .transferMintAuthorityToBoss()
            .accounts({
                mint: params.mint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async addAdmin(params: { admin: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods
            .addAdmin(params.admin);

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async removeAdmin(params: { admin: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods
            .removeAdmin(params.admin);

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async proposeBoss(params: { newBoss: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods
            .proposeBoss(params.newBoss)
            .accounts({
                boss: params.signer ? params.signer.publicKey : this.testHelper.payer.publicKey
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async acceptBoss(params: { newBoss: Keypair }) {
        const tx = this.program.methods
            .acceptBoss()
            .accounts({
                newBoss: params.newBoss.publicKey
            })
            .signers([params.newBoss]);

        await tx.rpc();
    }

    async setKillSwitch(params: { enable: boolean, signer?: Keypair }) {
        const tx = this.program.methods
            .setKillSwitch(params.enable)
            .accounts({
                signer: params.signer ? params.signer.publicKey : this.testHelper.payer.publicKey
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async setOnycMint(params: { onycMint: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods
            .setOnycMint()
            .accounts({
                onycMint: params.onycMint
            });

        if (params?.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async setRedemptionAdmin(params: { redemptionAdmin: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods
            .setRedemptionAdmin(params.redemptionAdmin)
            .accounts({});

        if (params?.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async makeRedemptionOffer(params: {
        offer: PublicKey;
        feeBasisPoints?: number;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        // Fetch the offer to get token mints
        const offer = await this.program.account.offer.fetch(params.offer);

        const tx = this.program.methods
            .makeRedemptionOffer(params.feeBasisPoints ?? 0)
            .accounts({
                tokenInMint: offer.tokenOutMint,
                tokenOutMint: offer.tokenInMint,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
                signer: params.signer ? params.signer.publicKey : this.testHelper.payer.publicKey
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async updateRedemptionOfferFee(params: {
        redemptionOffer: PublicKey;
        newFeeBasisPoints: number;
        signer?: Keypair;
    }) {
        const tx = this.program.methods
            .updateRedemptionOfferFee(params.newFeeBasisPoints)
            .accountsPartial({
                redemptionOffer: params.redemptionOffer,
                boss: params.signer ? params.signer.publicKey : this.testHelper.payer.publicKey
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async mintTo(params: { amount: number, signer?: Keypair }) {
        const tx = this.program.methods
            .mintTo(new BN(params.amount))
            .accounts({
                tokenProgram: TOKEN_PROGRAM_ID
            });

        if (params?.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async configureMaxSupply(params: { maxSupply: number, signer?: Keypair }) {
        const tx = this.program.methods
            .configureMaxSupply(new BN(params.maxSupply));

        if (params?.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async closeState(params?: { signer?: Keypair }) {
        const tx = this.program.methods
            .closeState().accounts({
                state: this.pdas.statePda
            });

        if (params?.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async getNAV(params: { tokenInMint: PublicKey, tokenOutMint: PublicKey }): Promise<number> {
        const tx = await this.program.methods
            .getNav()
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint
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

    async getAPY(params: { tokenInMint: PublicKey, tokenOutMint: PublicKey }): Promise<number> {
        const tx = await this.program.methods
            .getApy()
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint
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

    async getNavAdjustment(params: { tokenInMint: PublicKey, tokenOutMint: PublicKey }): Promise<number> {
        const tx = await this.program.methods
            .getNavAdjustment()
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint
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

    async getTVL(params: {
        tokenInMint: PublicKey,
        tokenOutMint: PublicKey,
        tokenOutProgram?: PublicKey
    }): Promise<BN> {
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;
        const tx = await this.program.methods
            .getTvl()
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                tokenOutProgram: tokenOutProgram,
                vaultTokenOutAccount: getAssociatedTokenAddressSync(params.tokenOutMint, this.pdas.offerVaultAuthorityPda, true, tokenOutProgram)
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

    async getCirculatingSupply(params: {
        onycMint: PublicKey,
        tokenOutProgram?: PublicKey
    }): Promise<BN> {
        const tokenOutProgram = params.tokenOutProgram ?? TOKEN_PROGRAM_ID;

        const tx = await this.program.methods
            .getCirculatingSupply()
            .accounts({
                tokenProgram: tokenOutProgram,
                onycVaultAccount: getAssociatedTokenAddressSync(params.onycMint, this.pdas.offerVaultAuthorityPda, true, tokenOutProgram)
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

    async getPermissionlessAuthority() {
        return await this.program.account.permissionlessAuthority.fetch(this.pdas.permissionlessAuthorityPda);
    }

    async addApprover(params: { trusted: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods.addApprover(params.trusted);

        if (params.signer) {
            tx.signers([params.signer]);
        }
        await tx.rpc();
    }

    async removeApprover(params: { approver: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods.removeApprover(params.approver);

        if (params.signer) {
            tx.signers([params.signer]);
        }
        await tx.rpc();
    }

    async createRedemptionRequest(params: {
        redemptionOffer: PublicKey;
        redeemer: Keypair;
        amount: number;
        tokenProgram?: PublicKey;
    }) {
        const redemptionOffer = await this.program.account.redemptionOffer.fetch(params.redemptionOffer);
        const tokenProgram = params.tokenProgram ?? TOKEN_PROGRAM_ID;

        const redeemerTokenAccount = getAssociatedTokenAddressSync(
            redemptionOffer.tokenInMint,
            params.redeemer.publicKey,
            false,
            tokenProgram
        );

        const vaultTokenAccount = getAssociatedTokenAddressSync(
            redemptionOffer.tokenInMint,
            this.pdas.redemptionVaultAuthorityPda,
            true,
            tokenProgram
        );

        const tx = this.program.methods
            .createRedemptionRequest(
                new BN(params.amount)
            )
            .accounts({
                redemptionOffer: params.redemptionOffer,
                redeemer: params.redeemer.publicKey,
                tokenInMint: redemptionOffer.tokenInMint,
                tokenProgram,
            })
            .signers([params.redeemer]);

        await tx.rpc();
    }

    async cancelRedemptionRequest(params: {
        redemptionOffer: PublicKey;
        redemptionRequest: PublicKey;
        signer: Keypair;
        redemptionAdmin: PublicKey;
        tokenProgram?: PublicKey;
    }) {
        const redemptionOffer = await this.program.account.redemptionOffer.fetch(params.redemptionOffer);
        const redemptionRequest = await this.program.account.redemptionRequest.fetch(params.redemptionRequest);
        const tokenProgram = params.tokenProgram ?? TOKEN_PROGRAM_ID;

        const tx = this.program.methods
            .cancelRedemptionRequest()
            .accounts({
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
    }) {
        let amount = params.amount;
        if (amount === undefined) {
            const request = await this.program.account.redemptionRequest.fetch(params.redemptionRequest);
            amount = (request.amount as BN).sub(request.fulfilledAmount as BN);
        }

        const tx = this.program.methods
            .fulfillRedemptionRequest(amount)
            .accounts({
                offer: params.offer,
                redemptionOffer: params.redemptionOffer,
                redemptionRequest: params.redemptionRequest,
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
                redeemer: params.redeemer,
                redemptionAdmin: params.redemptionAdmin.publicKey
            })
            .signers([params.redemptionAdmin]);

        await tx.rpc();
    }

    async getRedemptionRequest(redemptionOffer: PublicKey, counter: number) {
        const pda = this.getRedemptionRequestPda(redemptionOffer, counter);
        return await this.program.account.redemptionRequest.fetch(pda);
    }

    getRedemptionRequestPda(redemptionOffer: PublicKey, counter: number) {
        return PublicKey.findProgramAddressSync(
            [
                Buffer.from("redemption_request"),
                redemptionOffer.toBuffer(),
                new BN(counter).toArrayLike(Buffer, "le", 8)
            ],
            this.program.programId
        )[0];
    }
}