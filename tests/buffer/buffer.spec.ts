import { Keypair, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { TestHelper } from "../test_helper.ts";
import { OnreProgram } from "../onre_program.ts";

describe("BUFFER", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;
    let tokenInMint: PublicKey;
    let onycMint: PublicKey;
    let bufferAdmin: Keypair;
    let offerPda: PublicKey;

    const NAV_1_0 = 1_000_000_000;
    const ONE_YEAR_SECONDS = 31_536_000;

    // Fee model reference:
    //
    // 1. Gross accrual is computed from the spread on lowest supply.
    // 2. Management fee takes its APR slice first.
    // 3. Performance fee basis points apply only to the post-management remainder.
    // 4. Performance fee applies only when current NAV is above the stored NAV HWM.

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);
        tokenInMint = testHelper.createMint(6);
        onycMint = testHelper.createMint(9, null, BigInt(0));
        bufferAdmin = testHelper.createUserAccount();

        await program.initialize({ onycMint });

        await program.makeOffer({
            tokenInMint,
            tokenOutMint: onycMint,
            feeBasisPoints: 0,
            withApproval: false,
            allowPermissionless: true,
        });

        const now = await testHelper.getCurrentClockTime();
        offerPda = program.getOfferPda(tokenInMint, onycMint);
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

    async function setupBufferWithBalance(params?: {
        grossYield?: number;
        currentYieldApr?: number;
        managementFeeBasisPoints?: number;
        performanceFeeBasisPoints?: number;
        accrualPeriods?: number;
    }) {
        const { grossYield = 150_000, currentYieldApr = 50_000, managementFeeBasisPoints = 0, performanceFeeBasisPoints = 0, accrualPeriods = 1 } = params ?? {};

        const now = await testHelper.getCurrentClockTime();
        await program.deleteAllOfferVectors(tokenInMint, onycMint);
        await testHelper.advanceSlot()
        await program.addOfferVector({
            tokenInMint,
            tokenOutMint: onycMint,
            startTime: now,
            baseTime: now,
            basePrice: NAV_1_0,
            apr: currentYieldApr,
            priceFixDuration: 86_400,
        });

        await program.transferMintAuthorityToProgram({ mint: onycMint });
        await program.mintTo({ amount: 1_000_000_000 });
        await program.initializeBuffer({ offer: offerPda, onycMint, bufferAdmin: bufferAdmin.publicKey });
        await program.setBufferGrossYield({ grossYield });
        if (managementFeeBasisPoints !== 0 || performanceFeeBasisPoints !== 0) {
            await program.setBufferFeeConfig({
                managementFeeBasisPoints,
                performanceFeeBasisPoints,
            });
        }

        // First call sets lowestSupply from 0 -> current supply.
        await program.manageBuffer({ offer: offerPda, onycMint, signer: bufferAdmin });
        for (let i = 0; i < accrualPeriods; i++) {
            await testHelper.advanceSlot();
            await testHelper.advanceClockBy(ONE_YEAR_SECONDS);
            await program.manageBuffer({ offer: offerPda, onycMint, signer: bufferAdmin });
        }
    }

    test("initializes BUFFER state", async () => {
        await program.initializeBuffer({ offer: offerPda, onycMint, bufferAdmin: bufferAdmin.publicKey });
        const state = await program.getBufferState();

        expect(state.onycMint).toEqual(onycMint);
        expect(state.bufferAdmin).toEqual(bufferAdmin.publicKey);
        expect((await program.getState()).mainOffer).toEqual(offerPda);
        expect(state.grossApr.toNumber()).toBe(0);
        expect(state.managementFeeBasisPoints).toBe(0);
        expect(state.performanceFeeBasisPoints).toBe(0);
        expect(state.performanceFeeHighWatermark.toNumber()).toBe(0);
    });

    test("boss sets buffer admin, non-boss fails", async () => {
        await program.initializeBuffer({ offer: offerPda, onycMint, bufferAdmin: bufferAdmin.publicKey });

        const newAdmin = testHelper.createUserAccount();
        await program.setBufferAdmin({ bufferAdmin: newAdmin.publicKey });
        const updated = await program.getBufferState();
        expect(updated.bufferAdmin).toEqual(newAdmin.publicKey);

        const nonBoss = testHelper.createUserAccount();
        await expect(program.setBufferAdmin({ bufferAdmin: bufferAdmin.publicKey, signer: nonBoss })).rejects.toThrow();
    });

    test("accrues BUFFER mint to vault", async () => {
        await setupBufferWithBalance();
        const bufferVaultAta = program.getBufferVaultAta(onycMint);
        const vaultBalance = await testHelper.getTokenAccountBalance(bufferVaultAta);
        expect(vaultBalance).toBe(BigInt(100_000_000));
    });

    test("anyone can accrue", async () => {
        await setupBufferWithBalance();
        await testHelper.advanceSlot();
        await testHelper.advanceClockBy(ONE_YEAR_SECONDS);
        await program.manageBuffer({ offer: offerPda, onycMint });

        const bufferVaultAta = program.getBufferVaultAta(onycMint);
        const vaultBalance = await testHelper.getTokenAccountBalance(bufferVaultAta);
        expect(vaultBalance > BigInt(100_000_000)).toBe(true);
    });

    test("set buffer yields rejects no-change", async () => {
        await setupBufferWithBalance();
        await expect(program.setBufferGrossYield({ grossYield: 150_000 })).rejects.toThrow();
    });

    test("accrue buffer mints nothing when spread is zero", async () => {
        await setupBufferWithBalance({ grossYield: 50_000, currentYieldApr: 50_000 });

        const bufferVaultAta = program.getBufferVaultAta(onycMint);
        const vaultBalance = await testHelper.getTokenAccountBalance(bufferVaultAta);
        expect(vaultBalance).toBe(BigInt(0));
    });

    test("accrue buffer splits gross mint across buffer and fee vaults", async () => {
        await setupBufferWithBalance({
            managementFeeBasisPoints: 100,
            performanceFeeBasisPoints: 1_000,
        });

        const bufferVaultBalance = await testHelper.getTokenAccountBalance(program.getBufferVaultAta(onycMint));
        const managementFeeBalance = await testHelper.getTokenAccountBalance(program.getManagementFeeVaultAta(onycMint));
        const performanceFeeBalance = await testHelper.getTokenAccountBalance(program.getPerformanceFeeVaultAta(onycMint));
        const state = await program.getBufferState();

        expect(bufferVaultBalance).toBe(BigInt(81_000_000));
        expect(managementFeeBalance).toBe(BigInt(10_000_000));
        expect(performanceFeeBalance).toBe(BigInt(9_000_000));
        expect(state.performanceFeeHighWatermark.toNumber()).toBeGreaterThan(NAV_1_0);
    });

    test("claims fees from separate fee vaults", async () => {
        await setupBufferWithBalance({
            managementFeeBasisPoints: 100,
            performanceFeeBasisPoints: 1_000,
        });

        await program.withdrawManagementFees({ onycMint, amount: 400_000 });
        await program.withdrawPerformanceFees({ onycMint, amount: 900_000 });

        const managementFeeBalance = await testHelper.getTokenAccountBalance(program.getManagementFeeVaultAta(onycMint));
        const performanceFeeBalance = await testHelper.getTokenAccountBalance(program.getPerformanceFeeVaultAta(onycMint));
        const bossBalance = await testHelper.getTokenAccountBalance(getAssociatedTokenAddressSync(onycMint, testHelper.payer.publicKey));
        expect(managementFeeBalance).toBe(BigInt(9_600_000));
        expect(performanceFeeBalance).toBe(BigInt(8_100_000));
        expect(bossBalance).toBe(BigInt(1_001_300_000));
    });

    test("performance fee only applies after recovering the high watermark", async () => {
        await setupBufferWithBalance({
            grossYield: 100_000,
            currentYieldApr: 0,
            managementFeeBasisPoints: 100,
            performanceFeeBasisPoints: 1_000,
            accrualPeriods: 2,
        });
        await testHelper.advanceSlot();

        await program.burnForNavIncrease({
            tokenInMint,
            onycMint,
            assetAdjustmentAmount: 110_000_000,
            targetNav: NAV_1_0,
        });

        const stateAfterBurn = await program.getBufferState();
        expect(stateAfterBurn.performanceFeeHighWatermark.toNumber()).toBe(NAV_1_0);
        const performanceFeeBalanceBefore = await testHelper.getTokenAccountBalance(program.getPerformanceFeeVaultAta(onycMint));

        await testHelper.advanceSlot();
        await testHelper.advanceClockBy(ONE_YEAR_SECONDS);
        await testHelper.advanceSlot();
        await program.manageBuffer({ offer: offerPda, onycMint, signer: bufferAdmin });
        await testHelper.advanceSlot();
        await testHelper.getTokenAccountBalance(program.getBufferVaultAta(onycMint));
        const performanceFeeBalance = await testHelper.getTokenAccountBalance(program.getPerformanceFeeVaultAta(onycMint));
        const state = await program.getBufferState();

        expect(performanceFeeBalance).toBe(performanceFeeBalanceBefore);
        expect(state.performanceFeeHighWatermark.toNumber()).toBe(NAV_1_0);
    });

    test("burn for nav increase works and non-boss fails", async () => {
        await setupBufferWithBalance({ grossYield: 100_000, currentYieldApr: 0 });

        await program.burnForNavIncrease({
            tokenInMint,
            onycMint,
            assetAdjustmentAmount: 50_000_000,
            targetNav: NAV_1_0,
        });

        const bufferVaultAta = program.getBufferVaultAta(onycMint);
        const vaultBalance = await testHelper.getTokenAccountBalance(bufferVaultAta);
        const mintAfter = await testHelper.getMintInfo(onycMint);

        expect(vaultBalance).toBe(BigInt(50_000_000));
        expect(mintAfter.supply).toBe(BigInt(1_050_000_000));
        expect((await program.getBufferState()).lowestSupply.toNumber()).toBe(1_050_000_000);

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

    test("burn for nav increase settles pending accrual before burning", async () => {
        await setupBufferWithBalance({ grossYield: 100_000, currentYieldApr: 0 });
        await testHelper.advanceSlot();
        await testHelper.advanceClockBy(ONE_YEAR_SECONDS);

        await program.burnForNavIncrease({
            tokenInMint,
            onycMint,
            assetAdjustmentAmount: 50_000_000,
            targetNav: NAV_1_0,
        });

        const bufferVaultAta = program.getBufferVaultAta(onycMint);
        const vaultBalance = await testHelper.getTokenAccountBalance(bufferVaultAta);
        const mintAfter = await testHelper.getMintInfo(onycMint);
        const bufferState = await program.getBufferState();

        expect(vaultBalance).toBe(BigInt(160_000_000));
        expect(mintAfter.supply).toBe(BigInt(1_160_000_000));
        expect(bufferState.lowestSupply.toNumber()).toBe(1_160_000_000);
    });

    test("burn for nav increase rejects invalid parameters", async () => {
        await setupBufferWithBalance({ grossYield: 100_000, currentYieldApr: 0 });
        await testHelper.advanceSlot();

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
