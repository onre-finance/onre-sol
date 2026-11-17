import { LOCAL_RPC_URL, STUDIO_URL } from "./constants";
import { createRuntime, resolveAuthorityPath } from "./runtime";
import { anchorBuild, regressionFlag, requireSurfpoolRunning, setProgramAuthority, solanaProgramDeploy } from "./surfpool";
import { assertMainnetMarketsExist, assertTokenPrograms, ensureMainOffer, ensureMarketStats, fundUserStableBalances, overrideForkGovernance, syncForkRent } from "./setup";
import { configureBufferSmoke } from "./scenarios/buffer";
import { configureVaultRecipientsSmoke, withdrawNonZeroVaultsSmoke } from "./scenarios/configurable-vaults";
import { runOfferSmoke, runRedemptionSmoke } from "./scenarios/offers-redemptions";
import { configureUsdgPropAmmSmoke, runUsdgPropAmmSmoke } from "./scenarios/prop-amm-usdg";

async function main() {
    const authorityPath = resolveAuthorityPath();
    console.log("=== Surfpool mainnet-fork regression ===");
    console.log(`RPC:       ${LOCAL_RPC_URL}`);
    console.log(`Studio:    ${STUDIO_URL}`);
    console.log(`Authority: ${authorityPath}`);

    await requireSurfpoolRunning();
    await syncForkRent();
    anchorBuild();

    const runtime = createRuntime(authorityPath);
    await assertTokenPrograms(runtime);
    if (regressionFlag("SKIP_DEPLOY")) {
        console.log("Skipping program deploy because SURFPOOL_REGRESSION_SKIP_DEPLOY=1");
    } else {
        await setProgramAuthority(runtime.authority.publicKey);
        await solanaProgramDeploy(authorityPath);
    }

    await overrideForkGovernance(runtime);
    await assertMainnetMarketsExist(runtime);
    const mainOffer = await ensureMainOffer(runtime);
    await ensureMarketStats(runtime, mainOffer);

    await fundUserStableBalances(runtime);

    await runOfferSmoke(runtime, mainOffer);
    await runRedemptionSmoke(runtime, mainOffer);

    await configureBufferSmoke(runtime, mainOffer);
    await ensureMarketStats(runtime, mainOffer);
    await runOfferSmoke(runtime, mainOffer);
    await runRedemptionSmoke(runtime, mainOffer);

    const recipients = await configureVaultRecipientsSmoke(runtime);
    await runOfferSmoke(runtime, mainOffer);
    await withdrawNonZeroVaultsSmoke(runtime, recipients);

    await configureUsdgPropAmmSmoke(runtime);
    await runUsdgPropAmmSmoke(runtime, mainOffer);

    console.log("\n=== Surfpool regression passed ===");
    console.log(`Studio: ${STUDIO_URL}`);
}

try {
    await main();
    process.exit(0);
} catch (error) {
    console.error("\n=== Surfpool regression failed ===");
    console.error(error);
    process.exit(1);
}
