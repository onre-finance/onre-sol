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
        offerAccountPda: PublicKey;
        offerVaultAuthorityPda: PublicKey;
        permissionlessVaultAuthorityPda: PublicKey;
        mintAuthorityPda: PublicKey;
    } = {
        offerAccountPda: PublicKey.findProgramAddressSync([Buffer.from("offers")], PROGRAM_ID)[0],
        offerVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("offer_vault_authority")], PROGRAM_ID)[0],
        permissionlessVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], PROGRAM_ID)[0],
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

    async makeOffer(params: {
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        feeBasisPoints?: number;
        signer?: Keypair;
        tokenInProgram?: PublicKey;
    }) {
        const feeBasisPoints = params.feeBasisPoints ?? 0;
        const tx = this.program.methods
            .makeOffer(new BN(feeBasisPoints))
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

    async addOfferVector(params: {
        offerId: number,
        startTime: number,
        startPrice: number,
        apr: number,
        priceFixDuration: number,
        signer?: Keypair;
    }) {
        const tx = this.program.methods
            .addOfferVector(
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

    async closeOffer(params: { offerId: number, signer?: Keypair }) {
        const tx = this.program.methods
            .closeOffer(new BN(params.offerId))
            .accounts({
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async updateOfferFee(params: { offerId: number, newFee: number, signer?: Keypair }) {
        const tx = this.program.methods
            .updateOfferFee(new BN(params.offerId), new BN(params.newFee))
            .accounts({
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async deleteOfferVector(params: { offerId: number, vectorId: number, signer?: Keypair }) {
        const tx = this.program.methods
            .deleteOfferVector(new BN(params.offerId), new BN(params.vectorId))
            .accounts({
                state: this.statePda
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async takeOffer(params: {
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
            .takeOffer(new BN(params.offerId), new BN(params.tokenInAmount))
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

    async takeOfferPermissionless(params: {
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
            .takeOfferPermissionless(new BN(params.offerId), new BN(params.tokenInAmount))
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

    async offerVaultDeposit(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .offerVaultDeposit(new BN(params.amount))
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

    async offerVaultWithdraw(params: {
        amount: number,
        tokenMint: PublicKey,
        signer?: Keypair,
        tokenProgram?: PublicKey
    }) {
        const tx = this.program.methods
            .offerVaultWithdraw(new BN(params.amount))
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

    async transferMintAuthorityToProgram(params: { mint: PublicKey, signer?: Keypair, tokenProgram?: PublicKey }) {
        const tx = this.program.methods
            .transferMintAuthorityToProgram()
            .accounts({
                state: this.statePda,
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
                state: this.statePda,
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
            .addAdmin(params.admin)
            .accounts({
                state: this.statePda,
                boss: this.program.provider.publicKey // Always the actual boss from state
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async removeAdmin(params: { admin: PublicKey, signer?: Keypair }) {
        const tx = this.program.methods
            .removeAdmin(params.admin)
            .accounts({
                state: this.statePda,
                boss: this.program.provider.publicKey // Always the actual boss from state
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async setKillSwitch(params: { enable: boolean, signer?: Keypair }) {
        const tx = this.program.methods
            .setKillSwitch(params.enable)
            .accounts({
                signer: params.signer ? params.signer.publicKey : this.program.provider.publicKey
            });

        if (params.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async migrateState(params?: { signer?: Keypair }) {
        const tx = this.program.methods
            .migrateState()
            .accounts({
                state: this.statePda,
                boss: params?.signer ? params.signer.publicKey : this.program.provider.publicKey
            });

        if (params?.signer) {
            tx.signers([params.signer]);
        }

        await tx.rpc();
    }

    async getNAV(params: { offerId: number }): Promise<number> {
        try {
            // First try with view() for the return value
            const response: BN = await this.program.methods
                .getNav(new BN(params.offerId))
                .accounts({
                    offerAccount: this.pdas.offerAccountPda
                })
                .signers([this.program.provider.wallet.payer])
                .view();

            return response.toNumber();
        } catch (error) {
            // If view() fails with the null data error, try rpc() to get proper error messages
            // Use rpc() to get the proper anchor error
            await this.program.methods
                .getNav(new BN(params.offerId))
                .accounts({
                    offerAccount: this.pdas.offerAccountPda
                })
                .signers([this.program.provider.wallet.payer])
                .rpc();

            // If rpc doesn't throw, something unexpected happened
            throw new Error("Unexpected success from rpc after view failure");
        }
    }

    // Accounts
    async getOfferAccount() {
        const offerAccountPda = this.pdas.offerAccountPda;
        return await this.program.account.offerAccount.fetch(offerAccountPda);
    }

    async getOffer(offerId: number) {
        const offerAccount = await this.getOfferAccount();
        return offerAccount.offers.find(offer => offer.offerId.toNumber() === offerId);
    }

    async getState() {
        return await this.program.account.state.fetch(this.statePda);
    }

    async getKillSwitchState() {
        // Kill switch state is now part of the main State account
        const state = await this.getState();
        return { isKilled: state.isKilled };
    }
}