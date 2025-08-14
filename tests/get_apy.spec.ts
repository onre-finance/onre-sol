import { AddedProgram, startAnchor } from "solana-bankrun";
import { Keypair, PublicKey, Signer, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import { BN, getProvider, Program } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { OnreApp } from "../target/types/onre_app";
import idl from "../target/idl/onre_app.json";
import { ONREAPP_PROGRAM_ID } from "./test_helper";
import { TestHelper } from "./test_helper";

describe("get apy", () => {
    let testHelper: TestHelper;
    let sellTokenMint: PublicKey;
    let buyToken1Mint: PublicKey;
    let boss: PublicKey;
    let program: Program<OnreApp>;
    let provider: BankrunProvider;

    beforeAll(async () => {
        const programInfo: AddedProgram = {
            programId: ONREAPP_PROGRAM_ID,
            name: "onreapp",
        };

        const context = await startAnchor("", [programInfo], []);
        provider = new BankrunProvider(context);
        program = new Program<OnreApp>(idl, provider);

        testHelper = new TestHelper(context, program);
        boss = provider.wallet.publicKey;

        // Create mints
        sellTokenMint = testHelper.createMint(boss, BigInt("20000000000000000"), 9);
        buyToken1Mint = testHelper.createMint(boss, BigInt("20000000000000000"), 9);

        await program.methods.initialize().accounts({ boss }).rpc();
    });

    async function createOfferAndGetApy(
        sellStartAmount: number,
        sellEndAmount: number,
        buyAmount: number,
        durationSeconds: number,
        testName: string
    ): Promise<{ apy: number, success: boolean }> {
        const { offerId, offerPda } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0),
            buyToken1Mint, BigInt(0),
            boss, BigInt(buyAmount)
        );

        const currentTime = await testHelper.getCurrentClockTime();
        const offerStartTime = currentTime;
        const offerEndTime = currentTime + durationSeconds;

        try {
            await program.methods
                .makeOfferOne(
                    offerId,
                    new BN(buyAmount.toString()),
                    new BN(sellStartAmount.toString()),
                    new BN(sellEndAmount.toString()),
                    new BN(offerStartTime),
                    new BN(offerEndTime),
                    new BN(86400) // 1 day price fix duration
                )
                .accounts({
                    sellTokenMint: sellTokenMint,
                    buyToken1Mint: buyToken1Mint,
                    state: PublicKey.findProgramAddressSync([Buffer.from('state')], ONREAPP_PROGRAM_ID)[0],
                })
                .rpc();

            const getApyInstruction = await program.methods
                .getApy()
                .accounts({
                    offer: offerPda
                })
                .instruction();

            const tx = new VersionedTransaction(
                new TransactionMessage({
                    payerKey: boss,
                    recentBlockhash: testHelper.context.lastBlockhash,
                    instructions: [getApyInstruction],
                }).compileToLegacyMessage(),
            );
            tx.sign([provider.wallet.payer]);

            const result = await testHelper.context.banksClient.simulateTransaction(tx);
            console.log(result.meta.returnData.data)

            if (result.meta.returnData.data) {
                // Decode the returned u64 (8 bytes) in little-endian format
                const buffer = Buffer.from(result.meta.returnData.data);
                const apyBasisPoints = buffer.readBigUInt64LE(0);
                const apyPercent = Number(apyBasisPoints) / 100; // Convert basis points to percentage
                
                console.log(`${testName}: APY = ${apyPercent}% (${Number(apyBasisPoints)} bp)`);
                return { apy: apyPercent, success: true };
            }
            
            return { apy: 0, success: false };
        } catch (error) {
            console.error(`${testName} failed:`, error);
            return { apy: 0, success: false };
        }
    }

    test("APY calculations for 2-40% range with various time periods", async () => {
        // Base token amounts for consistent testing
        const buyTokenAmount = 20_000_000_000_000_000; // 20M tokens with 9 decimals
        
        // Test cases: [expectedApy%, durationDays, testName]
        const testCases = [
            // Low APY range (2-10%)
            { expectedApy: 2, days: 365, name: "2% APY over 1 year" },
            { expectedApy: 5, days: 365, name: "5% APY over 1 year" },
            { expectedApy: 10, days: 365, name: "10% APY over 1 year" },
            
            // Medium APY range (15-25%)
            { expectedApy: 15, days: 365, name: "15% APY over 1 year" },
            { expectedApy: 20, days: 365, name: "20% APY over 1 year" },
            { expectedApy: 25, days: 365, name: "25% APY over 1 year" },
            
            // High APY range (30-40%)
            { expectedApy: 30, days: 365, name: "30% APY over 1 year" },
            { expectedApy: 35, days: 365, name: "35% APY over 1 year" },
            { expectedApy: 40, days: 365, name: "40% APY over 1 year" },
            
            // Various time periods with 20% APY
            { expectedApy: 20, days: 7, name: "20% APY over 7 days" },
            { expectedApy: 20, days: 30, name: "20% APY over 30 days" },
            { expectedApy: 20, days: 90, name: "20% APY over 90 days" },
            { expectedApy: 20, days: 180, name: "20% APY over 180 days" },
            
            // Short-term high growth scenarios
            { expectedApy: 100, days: 7, name: "100% APY over 7 days (crisis scenario)" },
            { expectedApy: 50, days: 30, name: "50% APY over 30 days (high volatility)" },
            { expectedApy: 11.8, days: 15, name: "11.8% APY over 15 days (high volatility)" },
        ];

        for (const testCase of testCases) {
            // Calculate sell token amounts based on expected APY
            // Formula: end_nav = start_nav * (1 + expectedApy)^(days/365)
            const startNav = 1.0; // Normalized starting NAV
            const annualizationFactor = testCase.days / 365.0;
            const growthFactor = Math.pow(1 + testCase.expectedApy / 100, annualizationFactor);
            const endNav = startNav * growthFactor;
            
            // Convert to token amounts
            // sell_amount = nav * buy_amount
            const sellStartAmount = Math.floor(startNav * buyTokenAmount / 1000);
            const sellEndAmount = Math.floor(endNav * buyTokenAmount / 1000);
            const durationSeconds = testCase.days * 24 * 60 * 60;
            
            console.log(`\n--- ${testCase.name} ---`);
            console.log(`Expected APY: ${testCase.expectedApy}%`);
            console.log(`Duration: ${testCase.days} days (${durationSeconds} seconds)`);
            console.log(`Start NAV: ${startNav}, End NAV: ${endNav.toFixed(6)}`);
            console.log(`Sell start: ${sellStartAmount}, Sell end: ${sellEndAmount}`);
            console.log(`Buy amount: ${buyTokenAmount}`);
            
            const result = await createOfferAndGetApy(
                sellStartAmount,
                sellEndAmount,
                buyTokenAmount,
                durationSeconds,
                testCase.name
            );
            
            expect(result.success).toBe(true);
            
            // Allow for some tolerance in APY calculation due to fixed-point arithmetic
            const tolerance = Math.max(0.1, testCase.expectedApy * 0.02); // 2% tolerance or 0.1%, whichever is larger
            expect(Math.abs(result.apy - testCase.expectedApy)).toBeLessThan(tolerance);
            console.log(`-----------------------------------`);
        }
    });

    test("Edge cases and boundary conditions", async () => {
        const buyTokenAmount = 20_000_000_000_000_000;
        
        // Test case 1: Zero growth (0% APY)
        console.log("\n--- Testing 0% APY (no growth) ---");
        const zeroGrowthResult = await createOfferAndGetApy(
            buyTokenAmount, // Same start and end amounts
            buyTokenAmount,
            buyTokenAmount,
            365 * 24 * 60 * 60, // 1 year
            "0% APY (no growth)"
        );
        expect(zeroGrowthResult.success).toBe(true);
        expect(zeroGrowthResult.apy).toBe(0);
        
        // Test case 2: Very small growth over long period
        console.log("\n--- Testing 0.1% APY over 1 year ---");
        const startNav = 1.0;
        const endNav = startNav * 1.001; // 0.1% growth
        const smallGrowthResult = await createOfferAndGetApy(
            Math.floor(startNav * buyTokenAmount),
            Math.floor(endNav * buyTokenAmount),
            buyTokenAmount,
            365 * 24 * 60 * 60,
            "0.1% APY over 1 year"
        );
        expect(smallGrowthResult.success).toBe(true);
        expect(smallGrowthResult.apy).toBeCloseTo(0.1, 1);
        
        // Test case 3: Realistic high volatility scenarios
        console.log("\n--- Testing 1% APY over 1 year ---");
        const onePercentResult = await createOfferAndGetApy(
            buyTokenAmount,
            Math.floor(buyTokenAmount * 1.01),
            buyTokenAmount,
            365 * 24 * 60 * 60, // 1 year
            "1% APY over 1 year"
        );
        expect(onePercentResult.success).toBe(true);
        expect(onePercentResult.apy).toBeCloseTo(1.0, 1);
        
        // Test case 4: Short time period with realistic growth
        console.log("\n--- Testing 20% APY over 3 days ---");
        const shortTermResult = await createOfferAndGetApy(
            buyTokenAmount,
            Math.floor(buyTokenAmount * Math.pow(1.2, 3/365)), // 20% annualized over 3 days
            buyTokenAmount,
            3 * 24 * 60 * 60, // 3 days
            "20% APY over 3 days"
        );
        expect(shortTermResult.success).toBe(true);
        expect(shortTermResult.apy).toBeCloseTo(20.0, 1);
    });
});