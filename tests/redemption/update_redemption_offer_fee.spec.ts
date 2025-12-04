import { PublicKey } from "@solana/web3.js";
import { TestHelper } from "../test_helper";
import { OnreProgram } from "../onre_program.ts";

describe("Update redemption offer fee", () => {
    let testHelper: TestHelper;
    let program: OnreProgram;

    let usdcMint: PublicKey;
    let onycMint: PublicKey;
    let offerPda: PublicKey;
    let redemptionOfferPda: PublicKey;

    beforeEach(async () => {
        testHelper = await TestHelper.create();
        program = new OnreProgram(testHelper.context);

        // Create mints
        usdcMint = testHelper.createMint(6);
        onycMint = testHelper.createMint(9);

        // Initialize program
        await program.initialize({ onycMint });

        // Create offer
        await program.makeOffer({
            tokenInMint: usdcMint,
            tokenOutMint: onycMint
        });

        offerPda = program.getOfferPda(usdcMint, onycMint);

        // Add vector to offer
        const currentTime = await testHelper.getCurrentClockTime();
        await program.addOfferVector({
            tokenInMint: usdcMint,
            tokenOutMint: onycMint,
            baseTime: currentTime,
            basePrice: 1e9, // 1.0
            apr: 0,
            priceFixDuration: 86400
        });

        // Create redemption offer with initial fee
        await program.makeRedemptionOffer({
            offer: offerPda,
            feeBasisPoints: 100 // 1%
        });

        redemptionOfferPda = program.getRedemptionOfferPda(onycMint, usdcMint);
    });

    describe("Fee update", () => {
        test("Should successfully update redemption offer fee", async () => {
            // given
            const newFeeBasisPoints = 500; // 5%

            // Fetch initial fee
            let redemptionOffer = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            expect(redemptionOffer.feeBasisPoints).toBe(100);

            // when
            await program.updateRedemptionOfferFee({
                redemptionOffer: redemptionOfferPda,
                newFeeBasisPoints
            });

            // then
            redemptionOffer = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            expect(redemptionOffer.feeBasisPoints).toBe(newFeeBasisPoints);
        });

        test("Should update fee to 0 (no fee)", async () => {
            // given
            const newFeeBasisPoints = 0; // 0%

            // when
            await program.updateRedemptionOfferFee({
                redemptionOffer: redemptionOfferPda,
                newFeeBasisPoints
            });

            // then
            const redemptionOffer = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            expect(redemptionOffer.feeBasisPoints).toBe(0);
        });

        test("Should update fee to maximum allowed (100%)", async () => {
            // given
            const newFeeBasisPoints = 10000; // 100%

            // when
            await program.updateRedemptionOfferFee({
                redemptionOffer: redemptionOfferPda,
                newFeeBasisPoints
            });

            // then
            const redemptionOffer = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            expect(redemptionOffer.feeBasisPoints).toBe(10000);
        });

        test("Should update fee multiple times", async () => {
            // given - Update fee to 2%
            await program.updateRedemptionOfferFee({
                redemptionOffer: redemptionOfferPda,
                newFeeBasisPoints: 200
            });

            let redemptionOffer = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            expect(redemptionOffer.feeBasisPoints).toBe(200);

            // when - Update fee to 7.5%
            await program.updateRedemptionOfferFee({
                redemptionOffer: redemptionOfferPda,
                newFeeBasisPoints: 750
            });

            // then
            redemptionOffer = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            expect(redemptionOffer.feeBasisPoints).toBe(750);

            // when - Update fee to 3.33%
            await program.updateRedemptionOfferFee({
                redemptionOffer: redemptionOfferPda,
                newFeeBasisPoints: 333
            });

            // then
            redemptionOffer = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
            expect(redemptionOffer.feeBasisPoints).toBe(333);
        });
    });

    describe("Error cases", () => {
        test("Should fail when fee exceeds 10000 basis points", async () => {
            // given
            const newFeeBasisPoints = 10001; // 100.01%

            // when/then
            await expect(
                program.updateRedemptionOfferFee({
                    redemptionOffer: redemptionOfferPda,
                    newFeeBasisPoints
                })
            ).rejects.toThrow("Invalid fee");
        });

        test("Should fail when fee is way too high", async () => {
            // given
            const newFeeBasisPoints = 50000; // 500%

            // when/then
            await expect(
                program.updateRedemptionOfferFee({
                    redemptionOffer: redemptionOfferPda,
                    newFeeBasisPoints
                })
            ).rejects.toThrow("Invalid fee");
        });

        test("Should fail when non-boss tries to update", async () => {
            // given
            const nonBoss = testHelper.createUserAccount();

            // when/then
            await expect(
                program.updateRedemptionOfferFee({
                    redemptionOffer: redemptionOfferPda,
                    newFeeBasisPoints: 500,
                    signer: nonBoss
                })
            ).rejects.toThrow("Unauthorized");
        });
    });

    describe("Fee updates with different values", () => {
        test("Should handle fractional percentage fees correctly", async () => {
            const testCases = [
                { fee: 1, description: "0.01%" },
                { fee: 10, description: "0.1%" },
                { fee: 50, description: "0.5%" },
                { fee: 125, description: "1.25%" },
                { fee: 333, description: "3.33%" },
                { fee: 999, description: "9.99%" },
                { fee: 1234, description: "12.34%" },
                { fee: 5555, description: "55.55%" },
                { fee: 9999, description: "99.99%" }
            ];

            for (const testCase of testCases) {
                await program.updateRedemptionOfferFee({
                    redemptionOffer: redemptionOfferPda,
                    newFeeBasisPoints: testCase.fee
                });

                const redemptionOffer = await program.program.account.redemptionOffer.fetch(redemptionOfferPda);
                expect(redemptionOffer.feeBasisPoints).toBe(testCase.fee);
            }
        });
    });
});
