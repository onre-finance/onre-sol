import { Keypair, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { TestHelper } from "../test_helper.ts";
import { OnreProgram } from "../onre_program.ts";

describe("CACHE", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let tokenInMint: PublicKey;
    let onycMint: PublicKey;
    let cacheAdmin: Keypair;

    const NAV_1_0 = 1_000_000_000;
    const ONE_YEAR_SECONDS = 31_536_000;

    // Fee model reference:
    //
    // 1. Gross accrual is computed first from the cache spread formula.
    // 2. Management fee is carved out of that gross mint first.
    // 3. Performance fee is then computed against the cache reserve high-water mark,
    //    not as a flat percentage of the original gross mint.
    // 4. The remainder is minted to the cache vault.
    //
    // Example with gross mint = 100 and both fees = 1%:
    // - management fee = floor(100 * 1%) = 1
    // - remaining after management = 99
    // - if cache balance before accrual is exactly at the performance HWM:
    //   - performance profit above HWM after management leg = 99
    //   - performance fee = floor(99 * 1%) = 0
    //   - final split = 99 to cache, 1 to management, 0 to performance
    // - if cache balance was already above HWM, the performance fee can be non-zero
    //   because it is charged on the amount above the stored HWM, not on the raw 100.
    //
    // The stored HWM is the cache vault balance only. Fee vault balances are tracked
    // separately and do not contribute to the HWM.

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);
        tokenInMint = testHelper.createMint(6);
        onycMint = testHelper.createMint(9, null, BigInt(0));
        cacheAdmin = testHelper.createUserAccount();

        await program.initialize({ onycMint });

        await program.makeOffer({
            tokenInMint,
            tokenOutMint: onycMint,
            feeBasisPoints: 0,
            withApproval: false,
            allowPermissionless: true,
        });

        const now = await testHelper.getCurrentClockTime();
        await program.addOfferVector({
            tokenInMint,
            tokenOutMint: onycMint,
            startTime: now,
            baseTime: now,
            basePrice: NAV_1_0,
            apr: 0,
            priceFixDuration: 86_400,
        });
    });

    async function setupCacheWithBalance(params?: {
        grossYield?: number;
        currentYield?: number;
        managementFeeBasisPoints?: number;
        performanceFeeBasisPoints?: number;
        accrualPeriods?: number;
    }) {
        const { grossYield = 150_000, currentYield = 50_000, managementFeeBasisPoints = 0, performanceFeeBasisPoints = 0, accrualPeriods = 1 } = params ?? {};

        await program.transferMintAuthorityToProgram({ mint: onycMint });
        await program.mintTo({ amount: 1_000_000_000 });
        await program.initializeCache({ onycMint, cacheAdmin: cacheAdmin.publicKey });
        await program.setCacheYields({ grossYield, currentYield });
        if (managementFeeBasisPoints !== 0 || performanceFeeBasisPoints !== 0) {
            await program.setCacheFeeConfig({
                managementFeeBasisPoints,
                performanceFeeBasisPoints,
            });
        }

        // First call sets lowestSupply from 0 -> current supply.
        await program.accrueCache({ onycMint, signer: cacheAdmin });
        for (let i = 0; i < accrualPeriods; i++) {
            await testHelper.advanceSlot();
            await testHelper.advanceClockBy(ONE_YEAR_SECONDS);
            await testHelper.advanceSlot();
            await program.accrueCache({ onycMint, signer: cacheAdmin });
        }
    }

    test("initializes CACHE state", async () => {
        await program.initializeCache({ onycMint, cacheAdmin: cacheAdmin.publicKey });
        const state = await program.getCacheState();

        expect(state.onycMint).toEqual(onycMint);
        expect(state.cacheAdmin).toEqual(cacheAdmin.publicKey);
        expect(state.grossYield.toNumber()).toBe(0);
        expect(state.currentYield.toNumber()).toBe(0);
        expect(state.managementFeeBasisPoints).toBe(0);
        expect(state.performanceFeeBasisPoints).toBe(0);
        expect(state.performanceFeeHighWatermark.toNumber()).toBe(0);
        expect(state.totalManagementFeesAccrued.toNumber()).toBe(0);
        expect(state.totalManagementFeesClaimed.toNumber()).toBe(0);
        expect(state.totalPerformanceFeesAccrued.toNumber()).toBe(0);
        expect(state.totalPerformanceFeesClaimed.toNumber()).toBe(0);
    });

    test("boss sets cache admin, non-boss fails", async () => {
        await program.initializeCache({ onycMint, cacheAdmin: cacheAdmin.publicKey });

        const newAdmin = testHelper.createUserAccount();
        await program.setCacheAdmin({ cacheAdmin: newAdmin.publicKey });
        const updated = await program.getCacheState();
        expect(updated.cacheAdmin).toEqual(newAdmin.publicKey);

        const nonBoss = testHelper.createUserAccount();
        await expect(program.setCacheAdmin({ cacheAdmin: cacheAdmin.publicKey, signer: nonBoss })).rejects.toThrow();
    });

    test("accrues CACHE mint to vault", async () => {
        await setupCacheWithBalance();
        const cacheVaultAta = program.getCacheVaultAta(onycMint);
        const vaultBalance = await testHelper.getTokenAccountBalance(cacheVaultAta);
        expect(vaultBalance).toBe(BigInt(100_000_000));
    });

    test("non-cache-admin cannot accrue", async () => {
        await setupCacheWithBalance();
        const unauthorized = testHelper.createUserAccount();
        await expect(program.accrueCache({ onycMint, signer: unauthorized })).rejects.toThrow();
    });

    test("set cache yields rejects no-change", async () => {
        await setupCacheWithBalance();
        await expect(program.setCacheYields({ grossYield: 150_000, currentYield: 50_000 })).rejects.toThrow();
    });

    test("accrue cache mints nothing when spread is zero", async () => {
        await setupCacheWithBalance({ grossYield: 50_000, currentYield: 50_000 });

        const cacheVaultAta = program.getCacheVaultAta(onycMint);
        const vaultBalance = await testHelper.getTokenAccountBalance(cacheVaultAta);
        expect(vaultBalance).toBe(BigInt(0));
    });

    test("accrue cache splits gross mint across cache and fee vaults", async () => {
        await setupCacheWithBalance({
            managementFeeBasisPoints: 100,
            performanceFeeBasisPoints: 1_000,
        });

        const cacheVaultBalance = await testHelper.getTokenAccountBalance(program.getCacheVaultAta(onycMint));
        const managementFeeBalance = await testHelper.getTokenAccountBalance(program.getManagementFeeVaultAta(onycMint));
        const performanceFeeBalance = await testHelper.getTokenAccountBalance(program.getPerformanceFeeVaultAta(onycMint));
        const state = await program.getCacheState();

        expect(cacheVaultBalance).toBe(BigInt(89_100_000));
        expect(managementFeeBalance).toBe(BigInt(1_000_000));
        expect(performanceFeeBalance).toBe(BigInt(9_900_000));
        expect(state.performanceFeeHighWatermark.toNumber()).toBe(89_100_000);
        expect(state.totalManagementFeesAccrued.toNumber()).toBe(1_000_000);
        expect(state.totalPerformanceFeesAccrued.toNumber()).toBe(9_900_000);
    });

    test("claims fees from separate fee vaults", async () => {
        await setupCacheWithBalance({
            managementFeeBasisPoints: 100,
            performanceFeeBasisPoints: 1_000,
        });

        await program.claimManagementFees({ onycMint, amount: 400_000 });
        await program.claimPerformanceFees({ onycMint, amount: 900_000 });

        const managementFeeBalance = await testHelper.getTokenAccountBalance(program.getManagementFeeVaultAta(onycMint));
        const performanceFeeBalance = await testHelper.getTokenAccountBalance(program.getPerformanceFeeVaultAta(onycMint));
        const bossBalance = await testHelper.getTokenAccountBalance(getAssociatedTokenAddressSync(onycMint, testHelper.payer.publicKey));
        const state = await program.getCacheState();

        expect(managementFeeBalance).toBe(BigInt(600_000));
        expect(performanceFeeBalance).toBe(BigInt(9_000_000));
        expect(bossBalance).toBe(BigInt(1_001_300_000));
        expect(state.totalManagementFeesClaimed.toNumber()).toBe(400_000);
        expect(state.totalPerformanceFeesClaimed.toNumber()).toBe(900_000);
    });

    test("performance fee only applies after recovering the high watermark", async () => {
        await setupCacheWithBalance({
            managementFeeBasisPoints: 100,
            performanceFeeBasisPoints: 1_000,
            accrualPeriods: 2,
        });

        await program.burnForNavIncrease({
            tokenInMint,
            onycMint,
            assetAdjustmentAmount: 110_000_000,
            targetNav: NAV_1_0,
        });

        const stateAfterBurn = await program.getCacheState();
        expect(stateAfterBurn.performanceFeeHighWatermark.toNumber()).toBe(178_200_000);
        const performanceFeeBalanceBefore = await testHelper.getTokenAccountBalance(program.getPerformanceFeeVaultAta(onycMint));

        await testHelper.advanceSlot();
        await testHelper.advanceClockBy(ONE_YEAR_SECONDS);
        await testHelper.advanceSlot();
        await program.accrueCache({ onycMint, signer: cacheAdmin });

        const cacheVaultBalance = await testHelper.getTokenAccountBalance(program.getCacheVaultAta(onycMint));
        const performanceFeeBalance = await testHelper.getTokenAccountBalance(program.getPerformanceFeeVaultAta(onycMint));
        const state = await program.getCacheState();

        expect(cacheVaultBalance < BigInt(178_200_000)).toBe(true);
        expect(performanceFeeBalance).toBe(performanceFeeBalanceBefore);
        expect(state.totalPerformanceFeesAccrued.toNumber()).toBe(Number(performanceFeeBalanceBefore));
        expect(state.performanceFeeHighWatermark.toNumber()).toBe(178_200_000);
    });

    test("burn for nav increase works and non-boss fails", async () => {
        await setupCacheWithBalance();

        await program.burnForNavIncrease({
            tokenInMint,
            onycMint,
            assetAdjustmentAmount: 50_000_000,
            targetNav: NAV_1_0,
        });

        const cacheVaultAta = program.getCacheVaultAta(onycMint);
        const vaultBalance = await testHelper.getTokenAccountBalance(cacheVaultAta);
        const mintAfter = await testHelper.getMintInfo(onycMint);

        expect(vaultBalance).toBe(BigInt(50_000_000));
        expect(mintAfter.supply).toBe(BigInt(1_050_000_000));

        const nonBoss = testHelper.createUserAccount();
        await expect(
            program.burnForNavIncrease({
                tokenInMint,
                onycMint,
                assetAdjustmentAmount: 10_000_000,
                targetNav: NAV_1_0,
                signer: nonBoss,
            }),
        ).rejects.toThrow();
    });

    test("burn for nav increase rejects invalid parameters", async () => {
        await setupCacheWithBalance();

        await expect(
            program.burnForNavIncrease({
                tokenInMint,
                onycMint,
                assetAdjustmentAmount: 10_000_000,
                targetNav: 0,
            }),
        ).rejects.toThrow();

        await expect(
            program.burnForNavIncrease({
                tokenInMint,
                onycMint,
                assetAdjustmentAmount: 200_000_000,
                targetNav: NAV_1_0,
            }),
        ).rejects.toThrow();
    });
});
