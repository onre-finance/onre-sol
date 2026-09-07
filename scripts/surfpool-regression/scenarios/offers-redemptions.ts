import { PublicKey, SYSVAR_INSTRUCTIONS_PUBKEY, SystemProgram } from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";

import { ACTIVE_OFFERS, ACTIVE_REDEMPTIONS, MINTS, SMALL_ONYC_REDEMPTION_AMOUNT, SMALL_STABLE_AMOUNT } from "../constants";
import { assertBigIntEq, assertBigIntGt, assertPublicKeyEq, bnToBigInt, tokenBalance } from "../assertions";
import { ata, bufferAccounts, configurableVaultAta, tokenProgramFor } from "../accounts";
import { configurableVaultPda, offerPda, PDAS, redemptionOfferPda, redemptionRequestPda } from "../pdas";
import { bn, fetchNullable, SmokeRuntime, sendIxs } from "../runtime";

export async function runOfferSmoke(runtime: SmokeRuntime, mainOffer: PublicKey): Promise<void> {
    console.log("\n== Offer permissionless buys ==");

    for (const offer of ACTIVE_OFFERS) {
        const beforeOnyc = await tokenBalance(runtime, ata(MINTS.onyc, runtime.authority.publicKey));
        const ix = await runtime.program.methods
            .takeOfferPermissionless(bn(SMALL_STABLE_AMOUNT), null)
            .accountsPartial(takeOfferPermissionlessAccounts(runtime, offer.mint))
            .instruction();

        await sendIxs(runtime, `take ${offer.symbol}->ONYC permissionless`, [ix]);

        const afterOnyc = await tokenBalance(runtime, ata(MINTS.onyc, runtime.authority.publicKey));
        assertBigIntGt(afterOnyc, beforeOnyc, `${offer.symbol}->ONYC user ONYC balance`);
        const onChainOffer = await runtime.program.account.offer.fetch(offerPda(offer.mint, MINTS.onyc));
        assertPublicKeyEq(onChainOffer.tokenInMint, offer.mint, `${offer.symbol} offer tokenInMint after take`);
        assertPublicKeyEq(onChainOffer.tokenOutMint, MINTS.onyc, `${offer.symbol} offer tokenOutMint after take`);
    }
}

export async function runRedemptionSmoke(runtime: SmokeRuntime, mainOffer: PublicKey): Promise<void> {
    console.log("\n== Redemption create and fulfill ==");

    for (const redemption of ACTIVE_REDEMPTIONS) {
        const redemptionOffer = redemptionOfferPda(MINTS.onyc, redemption.mint);
        const offer = offerPda(redemption.mint, MINTS.onyc);
        const requestId = `surfpool-${redemption.symbol}`.padEnd(32, "0");
        const redemptionRequest = redemptionRequestPda(redemptionOffer, runtime.authority.publicKey, requestId);

        const beforeOut = await tokenBalance(runtime, ata(redemption.mint, runtime.authority.publicKey));
        const createIx = await runtime.program.methods
            .createRedemptionRequest(
                bn(SMALL_ONYC_REDEMPTION_AMOUNT),
                requestId,
            )
            .accountsPartial({
                state: PDAS.state,
                redemptionOffer,
                offer,
                redemptionRequest,
                redeemer: runtime.authority.publicKey,
                redemptionVaultAuthority: PDAS.redemptionVaultAuthority,
                tokenInMint: MINTS.onyc,
                redeemerTokenAccount: ata(MINTS.onyc, runtime.authority.publicKey),
                vaultTokenAccount: ata(MINTS.onyc, PDAS.redemptionVaultAuthority, true),
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
            })
            .instruction();
        await sendIxs(runtime, `create ONYC->${redemption.symbol} redemption request`, [createIx]);

        const created = await runtime.program.account.redemptionRequest.fetch(redemptionRequest);
        assertPublicKeyEq(created.offer, redemptionOffer, `${redemption.symbol} redemption request offer`);
        assertBigIntEq(bnToBigInt(created.amount), BigInt(SMALL_ONYC_REDEMPTION_AMOUNT), `${redemption.symbol} redemption request amount`);

        const fulfillIx = await runtime.program.methods
            .fulfillRedemptionRequest(bn(SMALL_ONYC_REDEMPTION_AMOUNT))
            .accountsPartial(fulfillRedemptionAccounts(runtime, redemption.mint, redemptionOffer, redemptionRequest, mainOffer))
            .instruction();
        await sendIxs(runtime, `fulfill ONYC->${redemption.symbol} redemption request`, [fulfillIx]);

        const afterOut = await tokenBalance(runtime, ata(redemption.mint, runtime.authority.publicKey));
        assertBigIntGt(afterOut, beforeOut, `${redemption.symbol} redemption output balance`);
        const closed = await fetchNullable(() => runtime.program.account.redemptionRequest.fetch(redemptionRequest));
        if (closed) {
            throw new Error(`${redemption.symbol} redemption request remained open after full fulfillment`);
        }
    }
}

function takeOfferPermissionlessAccounts(runtime: SmokeRuntime, assetMint: PublicKey) {
    const assetProgram = tokenProgramFor(assetMint);
    const onycProgram = tokenProgramFor(MINTS.onyc);
    const user = runtime.authority.publicKey;

    return {
        offer: offerPda(assetMint, MINTS.onyc),
        state: PDAS.state,
        boss: runtime.authority.publicKey,
        vaultAuthority: PDAS.offerVaultAuthority,
        vaultTokenInAccount: ata(assetMint, PDAS.offerVaultAuthority, true, assetProgram),
        vaultTokenOutAccount: ata(MINTS.onyc, PDAS.offerVaultAuthority, true, onycProgram),
        permissionlessAuthority: PDAS.permissionlessAuthority,
        permissionlessTokenInAccount: ata(assetMint, PDAS.permissionlessAuthority, true, assetProgram),
        permissionlessTokenOutAccount: ata(MINTS.onyc, PDAS.permissionlessAuthority, true, onycProgram),
        tokenInMint: assetMint,
        tokenInProgram: assetProgram,
        tokenOutMint: MINTS.onyc,
        tokenOutProgram: onycProgram,
        userTokenInAccount: ata(assetMint, user, false, assetProgram),
        userTokenOutAccount: ata(MINTS.onyc, user, false, onycProgram),
        bossTokenInAccount: ata(assetMint, runtime.authority.publicKey, false, assetProgram),
        mintAuthority: PDAS.mintAuthority,
        instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
        user,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
    };
}

function fulfillRedemptionAccounts(runtime: SmokeRuntime, assetMint: PublicKey, redemptionOffer: PublicKey, redemptionRequest: PublicKey, mainOffer: PublicKey) {
    const assetProgram = tokenProgramFor(assetMint);
    const onycProgram = tokenProgramFor(MINTS.onyc);

    return {
        state: PDAS.state,
        offer: offerPda(assetMint, MINTS.onyc),
        redemptionOffer,
        redemptionRequest,
        redemptionVaultAuthority: PDAS.redemptionVaultAuthority,
        vaultTokenInAccount: ata(MINTS.onyc, PDAS.redemptionVaultAuthority, true, onycProgram),
        vaultTokenOutAccount: ata(assetMint, PDAS.redemptionVaultAuthority, true, assetProgram),
        tokenInMint: MINTS.onyc,
        tokenInProgram: onycProgram,
        tokenOutMint: assetMint,
        tokenOutProgram: assetProgram,
        userTokenOutAccount: ata(assetMint, runtime.authority.publicKey, false, assetProgram),
        offerProceedsVault: configurableVaultPda("offerProceeds"),
        offerProceedsTokenInAccount: configurableVaultAta("offerProceeds", MINTS.onyc),
        offerFeeVault: configurableVaultPda("offerFee"),
        offerFeeTokenInAccount: configurableVaultAta("offerFee", MINTS.onyc),
        mintAuthority: PDAS.mintAuthority,
        redeemer: runtime.authority.publicKey,
        worker: runtime.authority.publicKey,
        bufferAccounts: bufferAccounts(),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        offerVaultAuthority: PDAS.offerVaultAuthority,
        offerVaultOnycAccount: ata(MINTS.onyc, PDAS.offerVaultAuthority, true, onycProgram),
        marketStats: PDAS.marketStats,
        circulatingSupplyExcludedBalance: PDAS.circulatingSupplyExcludedBalance,
        mainOffer,
    };
}
