import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { TestHelper, ONREAPP_PROGRAM_ID } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Market Stats PDA", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let tokenInMint: PublicKey;
    let onycMint: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper);

        tokenInMint = testHelper.createMint(6);
        onycMint = testHelper.createMint(9, null, BigInt(0));

        await program.initialize({ onycMint });
        await program.makeOffer({
            tokenInMint,
            tokenOutMint: onycMint,
            allowPermissionless: true
        });
        await program.initializePermissionlessAuthority({
            accountName: "market-stats"
        });

        testHelper.createTokenAccount(tokenInMint, program.pdas.offerVaultAuthorityPda, BigInt(0), true);
        testHelper.createTokenAccount(onycMint, program.pdas.offerVaultAuthorityPda, BigInt(3_000e9), true);
        testHelper.createTokenAccount(tokenInMint, program.pdas.permissionlessAuthorityPda, BigInt(0), true);
        testHelper.createTokenAccount(onycMint, program.pdas.permissionlessAuthorityPda, BigInt(0), true);
        testHelper.createTokenAccount(onycMint, testHelper.getBoss(), BigInt(10_000e9));
        testHelper.createTokenAccount(tokenInMint, testHelper.getBoss(), BigInt(0));

        await program.offerVaultDeposit({
            amount: 3_000e9,
            tokenMint: onycMint
        });

        const currentTime = await testHelper.getCurrentClockTime();
        await program.addOfferVector({
            tokenInMint,
            tokenOutMint: onycMint,
            baseTime: currentTime,
            basePrice: 1e9,
            apr: 36_500,
            priceFixDuration: 86400
        });
    });

    it("derives and reads the canonical market stats PDA after takeOffer", async () => {
        const user = testHelper.createUserAccount();
        testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(10_000e6), true);

        await program.takeOffer({
            tokenInAmount: 1_000_100,
            tokenInMint,
            tokenOutMint: onycMint,
            user: user.publicKey,
            signer: user
        });

        const expectedPda = PublicKey.findProgramAddressSync(
            [Buffer.from("market_stats")],
            ONREAPP_PROGRAM_ID
        )[0];

        expect(program.getMarketStatsPda().toBase58()).toBe(expectedPda.toBase58());

        const marketStats = await program.getMarketStats();
        expect(marketStats.bump).toBeDefined();
        expect(marketStats.nav.toNumber()).toBe(1_000_100_000);
        expect(marketStats.navAdjustment.toNumber()).toBe(1_000_100_000);
        expect(marketStats.circulatingSupply.toString()).toBe("0");
        expect(marketStats.tvl.toString()).toBe("0");
        expect(marketStats.lastUpdatedAt.toNumber()).toBeGreaterThan(0);
        expect(marketStats.lastUpdatedSlot.toNumber()).toBeGreaterThanOrEqual(0);
    });

    it("reads the same PDA after takeOfferPermissionless", async () => {
        const user = testHelper.createUserAccount();
        testHelper.createTokenAccount(tokenInMint, user.publicKey, BigInt(10_000e6), true);

        await program.takeOfferPermissionless({
            tokenInAmount: 1_000_100,
            tokenInMint,
            tokenOutMint: onycMint,
            user: user.publicKey,
            signer: user
        });

        const marketStats = await program.getMarketStats();
        expect(program.getMarketStatsPda().toBase58()).toBe(
            PublicKey.findProgramAddressSync([Buffer.from("market_stats")], ONREAPP_PROGRAM_ID)[0].toBase58()
        );
        expect(marketStats.nav.toNumber()).toBe(1_000_100_000);
        expect(marketStats.navAdjustment.toNumber()).toBe(1_000_100_000);
        expect(marketStats.circulatingSupply.toString()).toBe("0");
        expect(marketStats.tvl.toString()).toBe("0");
    });

    it("refreshes and re-reads the market stats PDA on idle days", async () => {
        const initialPda = program.getMarketStatsPda();

        await program.refreshMarketStats({
            tokenInMint,
            onycMint
        });

        const firstSnapshot = await program.getMarketStats();

        await testHelper.advanceClockBy(86400);
        await testHelper.advanceSlot();

        await program.refreshMarketStats({
            tokenInMint,
            onycMint
        });

        const refreshedSnapshot = await program.getMarketStats();
        const vaultAta = getAssociatedTokenAddressSync(onycMint, program.pdas.offerVaultAuthorityPda, true);

        expect(initialPda.toBase58()).toBe(
            PublicKey.findProgramAddressSync([Buffer.from("market_stats")], ONREAPP_PROGRAM_ID)[0].toBase58()
        );
        expect(refreshedSnapshot.nav.toNumber()).toBe(1_000_200_000);
        expect(refreshedSnapshot.navAdjustment.toNumber()).toBe(1_000_200_000);
        expect(refreshedSnapshot.circulatingSupply.toString()).toBe("0");
        expect(refreshedSnapshot.tvl.toString()).toBe("0");
        expect(refreshedSnapshot.lastUpdatedAt.toNumber()).toBeGreaterThan(firstSnapshot.lastUpdatedAt.toNumber());
        expect(refreshedSnapshot.lastUpdatedSlot.toNumber()).toBeGreaterThan(firstSnapshot.lastUpdatedSlot.toNumber());
        expect(vaultAta.toBase58()).toBe(
            getAssociatedTokenAddressSync(onycMint, program.pdas.offerVaultAuthorityPda, true).toBase58()
        );
    });
});
