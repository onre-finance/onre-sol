import { AddedProgram, startAnchor } from "solana-bankrun";
import { BankrunProvider } from "anchor-bankrun";
import { Program, BN } from "@coral-xyz/anchor";
import { PublicKey, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import { OnreApp } from "../target/types/onre_app";
import idl from "../target/idl/onre_app.json";
import { ONREAPP_PROGRAM_ID, TestHelper } from "./test_helper";

describe("get_circulating_supply", () => {
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
        sellTokenMint = testHelper.createMint(boss, BigInt("20000000000000000"), 9); // 20M tokens
        buyToken1Mint = testHelper.createMint(boss, BigInt("1000000000000000"), 9); // 1M tokens (this is what we track circulating supply for)

        // Create vault token account for boss with some tokens (this should reduce circulating supply)
        testHelper.createTokenAccount(buyToken1Mint, boss, BigInt("200000000000000")); // 200K tokens in vault

        await program.methods.initialize().accounts({ boss }).rpc();
    });

    async function createOfferAndGetCirculatingSupply(
        buyAmount: number,
        testName: string
    ): Promise<{ circulatingSupply: number, success: boolean }> {
        const { offerId, offerPda } = testHelper.createOneTokenOfferAccounts(
            sellTokenMint, BigInt(0),
            buyToken1Mint, BigInt(0),
            boss, BigInt(buyAmount)
        );

        const currentTime = await testHelper.getCurrentClockTime();
        const offerStartTime = currentTime;
        const offerEndTime = currentTime + 86400; // 1 day

        try {
            await program.methods
                .makeOfferOne(
                    offerId,
                    new BN(buyAmount.toString()),
                    new BN("20000000000000000"), // 20M sell tokens start
                    new BN("22000000000000000"), // 22M sell tokens end
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

            const getCirculatingSupplyInstruction = await program.methods
                .getCirculatingSupply()
                .accounts({
                    offer: offerPda,
                    vaultAuthority: boss,
                    tokenMint: buyToken1Mint,
                })
                .instruction();

            const tx = new VersionedTransaction(
                new TransactionMessage({
                    payerKey: boss,
                    recentBlockhash: testHelper.context.lastBlockhash,
                    instructions: [getCirculatingSupplyInstruction],
                }).compileToLegacyMessage(),
            );
            tx.sign([provider.wallet.payer]);

            const result = await testHelper.context.banksClient.simulateTransaction(tx);
            console.log(result.meta.returnData.data)

            if (result.meta.returnData.data) {
                // Decode the returned u64 (8 bytes) in little-endian format
                const buffer = Buffer.from(result.meta.returnData.data);
                const circulatingSupply = buffer.readBigUInt64LE(0);
                
                console.log(`${testName}: Circulating supply = ${Number(circulatingSupply)}`);
                return { circulatingSupply: Number(circulatingSupply), success: true };
            }
            
            return { circulatingSupply: 0, success: false };
        } catch (error) {
            console.error(`${testName} failed:`, error);
            return { circulatingSupply: 0, success: false };
        }
    }

    it("calculates circulating supply correctly", async () => {
        const buyTokenAmount = 100_000_000_000_000; // 100K tokens locked in offer
        
        const result = await createOfferAndGetCirculatingSupply(
            buyTokenAmount,
            "Basic circulating supply test"
        );
        
        expect(result.success).toBe(true);
        // Expected: 1.2M total supply (1M + 200K vault) - 200K in vault - 100K locked in offer = 900K circulating
        const vaultBalance = 200_000_000_000_000; // 200K in vault (added to supply)
        const totalSupply = 1_000_000_000_000_000 + vaultBalance; // 1M + 200K = 1.2M
        const expectedCirculating = totalSupply - vaultBalance - buyTokenAmount;
        expect(result.circulatingSupply).toBe(expectedCirculating);
    });

    it("handles different offer sizes", async () => {
        // Test with larger offer
        const largeOfferAmount = 500_000_000_000_000; // 500K tokens
        
        const result = await createOfferAndGetCirculatingSupply(
            largeOfferAmount,
            "Large offer test"
        );
        
        expect(result.success).toBe(true);
        // Expected: 1.2M total supply - 200K in vault - 500K locked = 500K circulating
        const vaultBalance = 200_000_000_000_000; // 200K in vault (added to supply)
        const totalSupply = 1_000_000_000_000_000 + vaultBalance; // 1M + 200K = 1.2M
        const expectedCirculating = totalSupply - vaultBalance - largeOfferAmount;
        expect(result.circulatingSupply).toBe(expectedCirculating);
    });

    it("handles small offer amounts", async () => {
        // Test with small offer
        const smallOfferAmount = 1_000_000_000; // 1 token
        
        const result = await createOfferAndGetCirculatingSupply(
            smallOfferAmount,
            "Small offer test"
        );
        
        expect(result.success).toBe(true);
        // Expected: 1.2M total supply - 200K in vault - 1 locked = 999,999 circulating
        const vaultBalance = 200_000_000_000_000; // 200K in vault (added to supply)
        const totalSupply = 1_000_000_000_000_000 + vaultBalance; // 1M + 200K = 1.2M
        const expectedCirculating = totalSupply - vaultBalance - smallOfferAmount;
        expect(result.circulatingSupply).toBe(expectedCirculating);
    });
});