import { Keypair, PublicKey } from "@solana/web3.js";
import { BN, Program } from "@coral-xyz/anchor";
import { Onreapp } from "../target/types/onreapp";
import { ONREAPP_PROGRAM_ID } from "./test_helper.ts";
import idl from "../target/idl/onreapp.json";
import { BankrunProvider } from "anchor-bankrun";
import { ProgramTestContext } from "solana-bankrun";
import { PROGRAM_ID } from "../scripts/script-commons.ts";

export class OnreProgram {
    program: Program<Onreapp>;
    statePda: PublicKey;

    pdas: {
        buyOfferAccountPda: PublicKey;
        buyOfferVaultAuthorityPda: PublicKey;
        permissionlessVaultAuthorityPda: PublicKey;
    } = {
        buyOfferAccountPda: PublicKey.findProgramAddressSync([Buffer.from("buy_offers")], PROGRAM_ID)[0],
        buyOfferVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("buy_offer_vault_authority")], PROGRAM_ID)[0],
        permissionlessVaultAuthorityPda: PublicKey.findProgramAddressSync([Buffer.from("permissionless-1")], PROGRAM_ID)[0]
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
    }) {
        const feeBasisPoints = params.feeBasisPoints ?? 0;
        const tx = this.program.methods
            .makeBuyOffer(new BN(feeBasisPoints))
            .accounts({
                tokenInMint: params.tokenInMint,
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
        signer?: Keypair
    }) {
        const tx = this.program.methods
            .takeBuyOffer(new BN(params.offerId), new BN(params.tokenInAmount))
            .accounts({
                state: this.statePda,
                boss: this.program.provider.publicKey,
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user
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
        signer?: Keypair
    }) {
        const tx = this.program.methods
            .takeBuyOfferPermissionless(new BN(params.offerId), new BN(params.tokenInAmount))
            .accounts({
                state: this.statePda,
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user
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

    async buyOfferVaultDeposit(params: { amount: number, tokenMint: PublicKey }) {
        await this.program.methods
            .buyOfferVaultDeposit(new BN(params.amount))
            .accounts({
                state: this.statePda,
                tokenMint: params.tokenMint
            })
            .rpc();
    }

    async initializePermissionlessAccount(params: { accountName: string }) {
        await this.program.methods
            .initializePermissionlessAccount(params.accountName)
            .accounts({
                state: this.statePda
            })
            .rpc();
    }

    async transferMintAuthorityToProgram(params: { mint: PublicKey }) {
        await this.program.methods
            .transferMintAuthorityToProgram()
            .accounts({
                state: this.statePda,
                mint: params.mint
            })
            .rpc();
    }

    async transferMintAuthorityToBoss(params: { mint: PublicKey }) {
        await this.program.methods
            .transferMintAuthorityToBoss()
            .accounts({
                state: this.statePda,
                mint: params.mint
            })
            .rpc();
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

}