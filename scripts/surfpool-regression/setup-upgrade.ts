import { LOCAL_RPC_URL, STUDIO_URL } from "./constants";
import { createRuntime, resolveAuthorityPath } from "./runtime";
import { anchorBuild, regressionFlag, requireSurfpoolRunning, setProgramAuthority, solanaProgramDeploy } from "./surfpool";
import { assertMainnetMarketsExist, assertTokenPrograms, ensureMainOffer, ensureMarketStats, overrideForkGovernance, syncForkRent } from "./setup";

async function main() {
    const authorityPath = resolveAuthorityPath();
    console.log("=== Surfpool setup and program upgrade ===");
    console.log(`RPC:       ${LOCAL_RPC_URL}`);
    console.log(`Studio:    ${STUDIO_URL}`);
    console.log(`Authority: ${authorityPath}`);

    await requireSurfpoolRunning(Number(process.env.SURFPOOL_REGRESSION_WAIT_MS ?? "90000"));
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

    console.log("\n=== Surfpool setup complete ===");
    console.log(`Authority: ${runtime.authority.publicKey.toBase58()}`);
    console.log(`Studio:    ${STUDIO_URL}`);
}

try {
    await main();
    process.exit(0);
} catch (error) {
    console.error("\n=== Surfpool setup failed ===");
    console.error(error);
    process.exit(1);
}
