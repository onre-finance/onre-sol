import { Connection, PublicKey, SYSVAR_RENT_PUBKEY, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";

import { ACTIVE_OFFERS, ACTIVE_REDEMPTIONS, MINTS, USER_STABLE_BALANCE } from "./constants";
import { assertBigIntGte, assertNumberGte, assertPublicKeyEq, assertTruthy, bnToBigInt, tokenBalance } from "./assertions";
import { ata, tokenProgramFor } from "./accounts";
import { offerPda, PDAS } from "./pdas";
import { bn, fetchNullable, SmokeRuntime } from "./runtime";
import { setTokenBalance, surfnetRpc } from "./surfpool";

/** Keep cached mainnet accounts valid under the fork's rent checks. Surfpool's
 * genesis Rent can differ from the datasource's current Rent parameters. */
export async function syncForkRent(): Promise<void> {
    const datasource = new Connection(process.env.SURFPOOL_DATASOURCE_RPC_URL || process.env.SOL_MAINNET_RPC_URL || "https://api.mainnet-beta.solana.com", "confirmed");
    const rent = await datasource.getAccountInfo(SYSVAR_RENT_PUBKEY);
    if (!rent) throw new Error("Datasource Rent sysvar is missing");
    await surfnetRpc("surfnet_setAccount", [SYSVAR_RENT_PUBKEY.toBase58(), {
        lamports: rent.lamports,
        owner: rent.owner.toBase58(),
        executable: rent.executable,
        data: rent.data.toString("hex"),
    }]);
    console.log("  ok fork Rent matches datasource");
}

export async function fundUserStableBalances(runtime: SmokeRuntime): Promise<void> {
    for (const offer of ACTIVE_OFFERS) {
        await setTokenBalance(runtime.authority.publicKey, offer.mint, USER_STABLE_BALANCE, tokenProgramFor(offer.mint));
        const balance = await tokenBalance(runtime, ata(offer.mint, runtime.authority.publicKey));
        assertBigIntGte(balance, BigInt(USER_STABLE_BALANCE), `${offer.symbol} user balance after fork funding`);
    }
}

export async function overrideForkGovernance(runtime: SmokeRuntime): Promise<void> {
    const account = await runtime.connection.getAccountInfo(PDAS.state);
    if (!account) {
        throw new Error(`State PDA missing: ${PDAS.state.toBase58()}`);
    }

    const state = await runtime.program.account.state.fetch(PDAS.state);
    const usdcMainOffer = offerPda(MINTS.usdc, MINTS.onyc);
    const mainOffer = state.mainOffer.equals(PublicKey.default) ? usdcMainOffer : state.mainOffer;

    const patchedState = {
        ...state,
        boss: runtime.authority.publicKey,
        worker: runtime.authority.publicKey,
        onycMint: MINTS.onyc,
        mainOffer,
    };
    const data = await runtime.program.coder.accounts.encode("state", patchedState);

    await surfnetRpc("surfnet_setAccount", [
        PDAS.state.toBase58(),
        {
            lamports: account.lamports,
            owner: account.owner.toBase58(),
            executable: account.executable,
            data: Buffer.from(data).toString("hex"),
        },
    ]);
    const updated = await runtime.program.account.state.fetch(PDAS.state);
    assertPublicKeyEq(updated.boss, runtime.authority.publicKey, "State.boss after fork governance override");
    assertPublicKeyEq(updated.worker, runtime.authority.publicKey, "State.worker after fork governance override");
    assertPublicKeyEq(updated.onycMint, MINTS.onyc, "State.onycMint after fork governance override");
    assertPublicKeyEq(updated.mainOffer, mainOffer, "State.mainOffer after fork governance override");
    console.log(`  ok fork State.boss/worker/main_offer -> ${runtime.authority.publicKey.toBase58()}`);
}

export async function ensureMainOffer(runtime: SmokeRuntime): Promise<PublicKey> {
    const state = await runtime.program.account.state.fetch(PDAS.state);
    const expected = offerPda(MINTS.usdc, MINTS.onyc);
    if (state.mainOffer.equals(expected)) {
        return expected;
    }
    await runtime.program.methods
        .setMainOffer()
        .accountsPartial({
            state: PDAS.state,
            boss: runtime.authority.publicKey,
            offer: expected,
        })
        .rpc();
    const updated = await runtime.program.account.state.fetch(PDAS.state);
    assertPublicKeyEq(updated.mainOffer, expected, "State.mainOffer after setMainOffer");
    console.log(`  ok main offer -> ${expected.toBase58()}`);
    return expected;
}

export async function assertMainnetMarketsExist(runtime: SmokeRuntime): Promise<void> {
    for (const offer of ACTIVE_OFFERS) {
        const pda = offerPda(offer.mint, MINTS.onyc);
        const account = await runtime.program.account.offer.fetch(pda);
        assertPublicKeyEq(account.tokenInMint, offer.mint, `${offer.symbol} offer tokenInMint`);
        assertPublicKeyEq(account.tokenOutMint, MINTS.onyc, `${offer.symbol} offer tokenOutMint`);
        assertTruthy(account.allowPermissionless === 1, `${offer.symbol} offer must allow permissionless execution`);
        assertTruthy(account.disabled === 0, `${offer.symbol} offer must be enabled`);
        assertNumberGte(account.vectors.filter((vector) => bnToBigInt(vector.basePrice) > 0n).length, 1, `${offer.symbol} offer active price vector count`);
        console.log(`  ok active offer ${offer.symbol}: ${pda.toBase58()}`);
    }
    for (const redemption of ACTIVE_REDEMPTIONS) {
        const pda = PublicKey.findProgramAddressSync([Buffer.from("redemption_offer"), MINTS.onyc.toBuffer(), redemption.mint.toBuffer()], runtime.program.programId)[0];
        const account = await runtime.program.account.redemptionOffer.fetch(pda);
        assertPublicKeyEq(account.offer, offerPda(redemption.mint, MINTS.onyc), `${redemption.symbol} redemption offer`);
        assertPublicKeyEq(account.tokenInMint, MINTS.onyc, `${redemption.symbol} redemption tokenInMint`);
        assertPublicKeyEq(account.tokenOutMint, redemption.mint, `${redemption.symbol} redemption tokenOutMint`);
        assertTruthy(account.disabled === 0, `${redemption.symbol} redemption must be enabled`);
        console.log(`  ok active redemption ONyc -> ${redemption.symbol}: ${pda.toBase58()}`);
    }
}

export async function ensureMarketStats(runtime: SmokeRuntime, mainOffer: PublicKey): Promise<void> {
    const before = await fetchNullable(() => runtime.program.account.marketStats.fetch(PDAS.marketStats));
    await runtime.program.methods
        .refreshMarketStats()
        .accountsPartial({
            mainOffer,
            tokenInMint: MINTS.usdc,
            state: PDAS.state,
            onycMint: MINTS.onyc,
            circulatingSupplyExcludedBalance: PDAS.circulatingSupplyExcludedBalance,
            marketStats: PDAS.marketStats,
            signer: runtime.authority.publicKey,
            systemProgram: SystemProgram.programId,
        })
        .rpc();
    const after = await runtime.program.account.marketStats.fetch(PDAS.marketStats);
    if (after.lastUpdatedSlot.lt(before?.lastUpdatedSlot ?? bn(0))) {
        throw new Error("MarketStats lastUpdatedSlot moved backwards");
    }
    assertBigIntGte(bnToBigInt(after.nav), 1n, "MarketStats.nav");
    assertBigIntGte(bnToBigInt(after.circulatingSupply), 1n, "MarketStats.circulatingSupply");
    assertBigIntGte(bnToBigInt(after.tvl), 1n, "MarketStats.tvl");
    console.log(`  ok market stats refreshed at slot ${after.lastUpdatedSlot.toString()}`);
}

export async function assertTokenPrograms(runtime: SmokeRuntime): Promise<void> {
    for (const mint of [MINTS.usdc, MINTS.usdt, MINTS.usdg, MINTS.onyc]) {
        const info = await runtime.connection.getAccountInfo(mint);
        if (!info?.owner.equals(TOKEN_PROGRAM_ID) && !info?.owner.equals(TOKEN_2022_PROGRAM_ID)) {
            throw new Error(`${mint.toBase58()} is not owned by SPL Token or Token-2022 on this fork`);
        }
    }
}
