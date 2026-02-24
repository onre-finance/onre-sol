import { Keypair, PublicKey } from "@solana/web3.js";
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

    async function setupCacheWithBalance(grossYield = 150_000, currentYield = 50_000) {
        await program.transferMintAuthorityToProgram({ mint: onycMint });
        await program.mintTo({ amount: 1_000_000_000 });
        await program.initializeCache({ onycMint, cacheAdmin: cacheAdmin.publicKey });
        await program.setCacheYields({ grossYield, currentYield });

        // First call sets lowestSupply from 0 -> current supply.
        await program.accrueCache({ onycMint, signer: cacheAdmin });
        await testHelper.advanceSlot();

        await testHelper.advanceClockBy(ONE_YEAR_SECONDS);
        await testHelper.advanceSlot();

        // Second call mints 10% of 1_000_000_000.
        await program.accrueCache({ onycMint, signer: cacheAdmin });
    }

    test("initializes CACHE state", async () => {
        await program.initializeCache({ onycMint, cacheAdmin: cacheAdmin.publicKey });
        const state = await program.getCacheState();

        expect(state.onycMint).toEqual(onycMint);
        expect(state.cacheAdmin).toEqual(cacheAdmin.publicKey);
        expect(state.grossYield.toNumber()).toBe(0);
        expect(state.currentYield.toNumber()).toBe(0);
    });

    test("boss sets cache admin, non-boss fails", async () => {
        await program.initializeCache({ onycMint, cacheAdmin: cacheAdmin.publicKey });

        const newAdmin = testHelper.createUserAccount();
        await program.setCacheAdmin({ cacheAdmin: newAdmin.publicKey });
        const updated = await program.getCacheState();
        expect(updated.cacheAdmin).toEqual(newAdmin.publicKey);

        const nonBoss = testHelper.createUserAccount();
        await expect(
            program.setCacheAdmin({ cacheAdmin: cacheAdmin.publicKey, signer: nonBoss })
        ).rejects.toThrow();
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
        await expect(
            program.accrueCache({ onycMint, signer: unauthorized })
        ).rejects.toThrow();
    });

    test("set cache yields rejects no-change", async () => {
        await setupCacheWithBalance();
        await expect(
            program.setCacheYields({ grossYield: 150_000, currentYield: 50_000 })
        ).rejects.toThrow();
    });

    test("accrue cache mints nothing when spread is zero", async () => {
        await setupCacheWithBalance(50_000, 50_000);

        const cacheVaultAta = program.getCacheVaultAta(onycMint);
        const vaultBalance = await testHelper.getTokenAccountBalance(cacheVaultAta);
        expect(vaultBalance).toBe(BigInt(0));
    });

    test("burn for nav increase works and non-boss fails", async () => {
        await setupCacheWithBalance();

        await program.burnForNavIncrease({
            tokenInMint,
            onycMint,
            assetAdjustmentAmount: 50_000_000,
            targetNav: NAV_1_0
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
                signer: nonBoss
            })
        ).rejects.toThrow();
    });

    test("burn for nav increase rejects invalid parameters", async () => {
        await setupCacheWithBalance();

        await expect(
            program.burnForNavIncrease({
                tokenInMint,
                onycMint,
                assetAdjustmentAmount: 10_000_000,
                targetNav: 0
            })
        ).rejects.toThrow();

        await expect(
            program.burnForNavIncrease({
                tokenInMint,
                onycMint,
                assetAdjustmentAmount: 200_000_000,
                targetNav: NAV_1_0
            })
        ).rejects.toThrow();
    });
});
