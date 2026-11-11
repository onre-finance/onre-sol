import { PublicKey, SYSVAR_INSTRUCTIONS_PUBKEY, SystemProgram } from "@solana/web3.js";
import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createAssociatedTokenAccountIdempotentInstruction,
    getAssociatedTokenAddressSync,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";

import { MINTS } from "./constants";
import { ConfigurableVaultName, configurableVaultPda, offerPda, PDAS, propAmmPairPda, redemptionOfferPda } from "./pdas";
import { SmokeRuntime } from "./runtime";

export function tokenProgramFor(mint: PublicKey): PublicKey {
    return mint.equals(MINTS.usdg) ? TOKEN_2022_PROGRAM_ID : TOKEN_PROGRAM_ID;
}

export function ata(mint: PublicKey, owner: PublicKey, allowOwnerOffCurve = false, tokenProgram = tokenProgramFor(mint)): PublicKey {
    return getAssociatedTokenAddressSync(mint, owner, allowOwnerOffCurve, tokenProgram);
}

export function configurableVaultAta(kind: ConfigurableVaultName, mint: PublicKey): PublicKey {
    return ata(mint, configurableVaultPda(kind), true, tokenProgramFor(mint));
}

export async function createUserAtas(runtime: SmokeRuntime, mints: PublicKey[]) {
    return mints.map((mint) =>
        createAssociatedTokenAccountIdempotentInstruction(
            runtime.authority.publicKey,
            ata(mint, runtime.authority.publicKey),
            runtime.authority.publicKey,
            mint,
            tokenProgramFor(mint),
        ),
    );
}

export function bufferAccounts() {
    return {
        bufferState: PDAS.bufferState,
        reserveVaultOnycAccount: ata(MINTS.onyc, PDAS.reserveVaultAuthority, true),
        managementFeeVaultOnycAccount: configurableVaultAta("managementFee", MINTS.onyc),
        performanceFeeVaultOnycAccount: configurableVaultAta("performanceFee", MINTS.onyc),
    };
}

export function marketAccounts() {
    return {
        marketStats: PDAS.marketStats,
        circulatingSupplyExcludedBalance: PDAS.circulatingSupplyExcludedBalance,
    };
}

export function buyAccounts(assetMint: PublicKey, user: PublicKey, mainOffer: PublicKey) {
    const offer = offerPda(assetMint, MINTS.onyc);
    const assetProgram = tokenProgramFor(assetMint);
    const onycProgram = tokenProgramFor(MINTS.onyc);
    return {
        offer,
        propAmmPairState: propAmmPairPda(offer),
        redemptionOffer: redemptionOfferPda(MINTS.onyc, assetMint),
        state: PDAS.state,
        vaultAuthority: PDAS.offerVaultAuthority,
        offerVaultAuthority: PDAS.offerVaultAuthority,
        redemptionVaultAuthority: PDAS.redemptionVaultAuthority,
        offerVaultTokenInAccount: ata(assetMint, PDAS.offerVaultAuthority, true, assetProgram),
        offerVaultTokenOutAccount: ata(MINTS.onyc, PDAS.offerVaultAuthority, true, onycProgram),
        vaultTokenInAccount: ata(assetMint, PDAS.offerVaultAuthority, true, assetProgram),
        vaultTokenOutAccount: ata(MINTS.onyc, PDAS.offerVaultAuthority, true, onycProgram),
        redemptionVaultTokenInAccount: ata(assetMint, PDAS.redemptionVaultAuthority, true, assetProgram),
        tokenInMint: assetMint,
        tokenInProgram: assetProgram,
        tokenOutMint: MINTS.onyc,
        tokenOutProgram: onycProgram,
        userTokenInAccount: ata(assetMint, user, false, assetProgram),
        userTokenOutAccount: ata(MINTS.onyc, user, false, onycProgram),
        offerProceedsVault: configurableVaultPda("offerProceeds"),
        offerProceedsTokenInAccount: configurableVaultAta("offerProceeds", assetMint),
        offerFeeVault: configurableVaultPda("offerFee"),
        offerFeeTokenInAccount: configurableVaultAta("offerFee", assetMint),
        propAmmProceedsVault: configurableVaultPda("propAmmProceeds"),
        propAmmProceedsTokenInAccount: configurableVaultAta("propAmmProceeds", assetMint),
        propAmmBuyFeeVault: configurableVaultPda("propAmmBuyFee"),
        propAmmBuyFeeTokenInAccount: configurableVaultAta("propAmmBuyFee", assetMint),
        permissionlessAuthority: PDAS.permissionlessAuthority,
        permissionlessTokenInAccount: ata(assetMint, PDAS.permissionlessAuthority, true, assetProgram),
        permissionlessTokenOutAccount: ata(MINTS.onyc, PDAS.permissionlessAuthority, true, onycProgram),
        mintAuthority: PDAS.mintAuthority,
        bufferAccounts: bufferAccounts(),
        ...marketAccounts(),
        instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
        user,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        mainOffer,
    };
}

export function sellAccounts(assetMint: PublicKey, user: PublicKey, mainOffer: PublicKey) {
    const offer = offerPda(assetMint, MINTS.onyc);
    const assetProgram = tokenProgramFor(assetMint);
    const onycProgram = tokenProgramFor(MINTS.onyc);
    return {
        ...buyAccounts(assetMint, user, mainOffer),
        tokenInMint: MINTS.onyc,
        tokenOutMint: assetMint,
        tokenInProgram: onycProgram,
        tokenOutProgram: assetProgram,
        userTokenInAccount: ata(MINTS.onyc, user, false, onycProgram),
        userTokenOutAccount: ata(assetMint, user, false, assetProgram),
        redemptionVaultTokenInAccount: ata(MINTS.onyc, PDAS.redemptionVaultAuthority, true, onycProgram),
        redemptionVaultTokenOutAccount: ata(assetMint, PDAS.redemptionVaultAuthority, true, assetProgram),
        propAmmProceedsTokenInAccount: configurableVaultAta("propAmmProceeds", MINTS.onyc),
        propAmmSellFeeVault: configurableVaultPda("propAmmSellFee"),
        propAmmSellFeeTokenInAccount: configurableVaultAta("propAmmSellFee", MINTS.onyc),
        offerVaultOnycAccount: ata(MINTS.onyc, PDAS.offerVaultAuthority, true, onycProgram),
    };
}
