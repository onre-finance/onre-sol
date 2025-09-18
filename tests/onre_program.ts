import { Keypair, PublicKey } from "@solana/web3.js";
import { BN, Program } from "@coral-xyz/anchor";
import { Onreapp } from "../target/types/onreapp";
import { ONREAPP_PROGRAM_ID } from "./test_helper.ts";
import idl from "../target/idl/onreapp.json";
import { BankrunProvider } from "anchor-bankrun";
import { ProgramTestContext } from "solana-bankrun";
import { PROGRAM_ID } from "../scripts/script-commons.ts";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

export class OnreProgram {
    program: Program<Onreapp>;
    statePda: PublicKey;

    pdas: {
        buyOfferAccountPda: PublicKey;
        buyOfferVaultAuthorityPda: PublicKey;
        permissionlessVaultAuthorityPda: PublicKey;
        adminStatePda: PublicKey;
        dualRedemptionOfferAccountPda: PublicKey;
        dualRedemptionVaultAuthorityPda: PublicKey;
        singleRedemptionOfferAccountPda: PublicKey;
        singleRedemptionVaultAuthorityPda: PublicKey;
        mintAuthorityPda: PublicKey;
    } = {
        buyOfferAccountPda: PublicKey.findProgramAddressSync([Buffer.from("buy_offers")], PROGRAM_ID)[0],
        buyOfferVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("buy_offer_vault_authority")], PROGRAM_ID)[0],
        permissionlessVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], PROGRAM_ID)[0],
        adminStatePda: PublicKey.findProgramAddressSync([Buffer.from("admin_state")], PROGRAM_ID)[0],
        dualRedemptionOfferAccountPda: PublicKey.findProgramAddressSync([Buffer.from("dual_redemption_offers")], PROGRAM_ID)[0],
        dualRedemptionVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("dual_redemption_vault_auth")], PROGRAM_ID)[0],
        singleRedemptionOfferAccountPda: PublicKey.findProgramAddressSync([Buffer.from("single_redemption_offers")], PROGRAM_ID)[0],
        singleRedemptionVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("single_redemption_vault_auth")], PROGRAM_ID)[0],
        mintAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("mint_authority")], PROGRAM_ID)[0]
    };

    constructor(context: ProgramTestContext) {
        const provider = new BankrunProvider(context);

        this.program = new Program<Onreapp>(
            idl,
            provider
        );
        [this.statePda] = PublicKey.findProgramAddressSync([Buffer.from("state")], ONREAPP_PROGRAM_ID);
    }

    // Instructions
    async initialize() {
        await this.program.methods
            .initialize()
            .accounts({
                boss: this.program.provider.publicKey
            })
            .rpc();
    }

    async initializeOffers() {
        await this.program.methods.initializeOffers().accounts({
            state: this.statePda
        }).rpc();
    }

    async makeBuyOffer(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        feeBasisPoints?: number;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
    }) {
        const feeBasisPoints = params.feeBasisPoints ?? 0;
        const tx = this.program.methods
            .makeBuyOffer(new BN(feeBasisPoints))
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutMint: params.tokenOutMint,
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async addBuyOfferVector(params: {
        offerId: number,
        startTime: number,
        startPrice: number,
        apr: number,
        priceFixDuration: number,
        signer?: Keypair;
    }) {
        const tx = this.program.methods
            .addBuyOfferVector(
                new BN(params.offerId),
                new BN(params.startTime),
                new BN(params.startPrice),
                new BN(params.apr),
                new BN(params.priceFixDuration)
            )
            .accounts({
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async closeBuyOffer(params: { offerId: number, signer?: Keypair }) {
        const tx = this.program.methods
            .closeBuyOffer(new BN(params.offerId))
            .accounts({
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async updateBuyOfferFee(params: { offerId: number, newFee: number, signer?: Keypair }) {
        const tx = this.program.methods
            .updateBuyOfferFee(new BN(params.offerId), new BN(params.newFee))
            .accounts({
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async deleteBuyOfferVector(params: { offerId: number, vectorId: number, signer?: Keypair }) {
        const tx = this.program.methods
            .deleteBuyOfferVector(new BN(params.offerId), new BN(params.vectorId))
            .accounts({
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async takeBuyOffer(params: {
        offerId: number,
        tokenInAmount: number,
        tokenInMint: PublicKey,
        tokenOutMint: PublicKey,
        user: PublicKey,
        signer?: Keypair,
        tokenInProgram?: PublicKey,
        tokenOutProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .takeBuyOffer(new BN(params.offerId), new BN(params.tokenInAmount))
            .accounts({
                state: this.statePda,
                boss: this.program.provider.publicKey,
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

    async takeBuyOfferPermissionless(params: {
        offerId: number,
        tokenInAmount: number,
        tokenInMint: PublicKey,
        tokenOutMint: PublicKey,
        user: PublicKey,
        signer?: Keypair,
        tokenInProgram?: PublicKey,
        tokenOutProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .takeBuyOfferPermissionless(new BN(params.offerId), new BN(params.tokenInAmount))
            .accounts({
                state: this.statePda,
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

    async initializeVaultAuthority() {
        await this.program.methods
            .initializeVaultAuthority()
            .accounts({
                state: this.statePda
            })
            .rpc();
    }

    async buyOfferVaultDeposit(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .buyOfferVaultDeposit(new BN(params.amount))
            .accounts({
                state: this.statePda,
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async singleRedemptionVaultDeposit(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .singleRedemptionVaultDeposit(new BN(params.amount))
            .accounts({
                state: this.statePda,
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async dualRedemptionVaultDeposit(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .dualRedemptionVaultDeposit(new BN(params.amount))
            .accounts({
                state: this.statePda,
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async buyOfferVaultWithdraw(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .buyOfferVaultWithdraw(new BN(params.amount))
            .accounts({
                state: this.statePda,
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async singleRedemptionVaultWithdraw(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .singleRedemptionVaultWithdraw(new BN(params.amount))
            .accounts({
                state: this.statePda,
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async dualRedemptionVaultWithdraw(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .dualRedemptionVaultWithdraw(new BN(params.amount))
            .accounts({
                state: this.statePda,
                tokenMint: params.tokenMint,
                tokenProgram: params.tokenProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async initializePermissionlessAccount(params: { accountName: string }) {
        await this.program.methods
            .initializePermissionlessAccount(params.accountName)
            .accounts({
                state: this.statePda
            })
            .rpc();
    }

    async transferMintAuthorityToProgram(params: { mint: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods
            .transferMintAuthorityToProgram()
            .accounts({
                state: this.statePda,
                mint: params.mint
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async transferMintAuthorityToBoss(params: { mint: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods
            .transferMintAuthorityToBoss()
            .accounts({
                state: this.statePda,
                mint: params.mint
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async initializeAdminState() {
        await this.program.methods
            .initializeAdminState()
            .accounts({
                state: this.statePda
            })
            .rpc();
    }

    async addAdmin(params: { admin: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods
            .addAdmin(params.admin)
            .accounts({});

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async removeAdmin(params: { admin: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods
            .removeAdmin(params.admin)
            .accounts({});

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async makeDualRedemptionOffer(params: {
        startTime: number;
        endTime: number;
        price1: number;
        price2: number;
        ratioBasisPoints: number;
        feeBasisPoints?: number;
        tokenInMint: PublicKey;
        tokenOutMint1: PublicKey;
        tokenOutMint2: PublicKey;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
    }) {
        const feeBasisPoints = params.feeBasisPoints ?? 0;
        const tx = this.program.methods
            .makeDualRedemptionOffer(
                new BN(params.startTime),
                new BN(params.endTime),
                new BN(params.price1),
                new BN(params.price2),
                new BN(params.ratioBasisPoints),
                new BN(feeBasisPoints)
            )
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint1: params.tokenOutMint1,
                tokenOutMint2: params.tokenOutMint2,
                state: this.statePda,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async closeDualRedemptionOffer(params: { offerId: number, signer?: Keypair }) {
        const tx = this.program.methods
            .closeDualRedemptionOffer(new BN(params.offerId))
            .accounts({
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async updateDualRedemptionOfferFee(params: { offerId: number, newFee: number, signer?: Keypair }) {
        const tx = this.program.methods
            .updateDualRedemptionOfferFee(new BN(params.offerId), new BN(params.newFee))
            .accounts({
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async takeDualRedemptionOffer(params: {
        offerId: number;
        tokenInAmount: number;
        tokenInMint: PublicKey;
        tokenOutMint1: PublicKey;
        tokenOutMint2: PublicKey;
        user: PublicKey;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
        tokenOutProgram1?: PublicKey;
        tokenOutProgram2?: PublicKey;
    }) {
        const tx = this.program.methods
            .takeDualRedemptionOffer(new BN(params.offerId), new BN(params.tokenInAmount))
            .accounts({
                state: this.statePda,
                boss: this.program.provider.publicKey,
                tokenInMint: params.tokenInMint,
                tokenOutMint1: params.tokenOutMint1,
                tokenOutMint2: params.tokenOutMint2,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram1: params.tokenOutProgram1 ?? TOKEN_PROGRAM_ID,
                tokenOutProgram2: params.tokenOutProgram2 ?? TOKEN_PROGRAM_ID,
                user: params.user
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    // Accounts
    async getBuyOfferAccount() {
        const buyOfferAccountPda = this.pdas.buyOfferAccountPda;
        return await this.program.account.buyOfferAccount.fetch(buyOfferAccountPda);
    }

    async getOffer(offerId: number) {
        const buyOfferAccount = await this.getBuyOfferAccount();
        return buyOfferAccount.offers.find(offer => offer.offerId.toNumber() === offerId);
    }

    async getAdminState() {
        return await this.program.account.adminState.fetch(this.pdas.adminStatePda);
    }

    async getDualRedemptionOfferAccount() {
        return await this.program.account.dualRedemptionOfferAccount.fetch(this.pdas.dualRedemptionOfferAccountPda);
    }

    async getDualRedemptionOffer(offerId: number) {
        const dualRedemptionOfferAccount = await this.getDualRedemptionOfferAccount();
        return dualRedemptionOfferAccount.offers.find(offer => offer.offerId.toNumber() === offerId);
    }

    // Single Redemption Offer methods
    async makeSingleRedemptionOffer(params: {
        startTime: number;
        endTime: number;
        price: number;
        feeBasisPoints?: number;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
    }) {
        const feeBasisPoints = params.feeBasisPoints ?? 0;
        const tx = this.program.methods
            .makeSingleRedemptionOffer(
                new BN(params.startTime),
                new BN(params.endTime),
                new BN(params.price),
                new BN(feeBasisPoints)
            )
            .accounts({
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                state: this.statePda,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async closeSingleRedemptionOffer(params: { offerId: number, signer?: Keypair }) {
        const tx = this.program.methods
            .closeSingleRedemptionOffer(new BN(params.offerId))
            .accounts({
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async updateSingleRedemptionOfferFee(params: { offerId: number, newFee: number, signer?: Keypair }) {
        const tx = this.program.methods
            .updateSingleRedemptionOfferFee(new BN(params.offerId), new BN(params.newFee))
            .accounts({
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async takeSingleRedemptionOffer(params: {
        offerId: number;
        tokenInAmount: number;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
        tokenOutProgram?: PublicKey;
    }) {
        const tx = this.program.methods
            .takeSingleRedemptionOffer(new BN(params.offerId), new BN(params.tokenInAmount))
            .accounts({
                state: this.statePda,
                boss: this.program.provider.publicKey,
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                tokenInProgram: params.tokenInProgram ?? TOKEN_PROGRAM_ID,
                tokenOutProgram: params.tokenOutProgram ?? TOKEN_PROGRAM_ID,
                user: params.user
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    // Single Redemption Offer Account getters
    async getSingleRedemptionOfferAccount() {
        return await this.program.account.singleRedemptionOfferAccount.fetch(this.pdas.singleRedemptionOfferAccountPda);
    }

    async getSingleRedemptionOffer(offerId: number) {
        const singleRedemptionOfferAccount = await this.getSingleRedemptionOfferAccount();
        return singleRedemptionOfferAccount.offers.find(offer => offer.offerId.toNumber() === offerId);
    }

}