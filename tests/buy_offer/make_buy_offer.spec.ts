import {PublicKey} from "@solana/web3.js";
import {ONREAPP_PROGRAM_ID, TestHelper} from "../test_helper";
import {AddedProgram, startAnchor} from "solana-bankrun";
import {Onreapp} from "../../target/types/onreapp";
import {BankrunProvider} from "anchor-bankrun";
import {Program} from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

const MAX_BUY_OFFERS = 10;

describe("Make buy offer", () => {
    let testHelper: TestHelper;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    let boss: PublicKey;

    beforeEach(async () => {
        const programInfo: AddedProgram = {
            programId: ONREAPP_PROGRAM_ID,
            name: "onreapp"
        };

        const workspace = process.cwd();
        const context = await startAnchor(workspace, [programInfo], []);

        const provider = new BankrunProvider(context);
        const program = new Program<Onreapp>(
            idl,
            provider
        );

        testHelper = new TestHelper(context, program);

        boss = provider.wallet.publicKey;

        // Create mints
        tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        tokenOutMint = testHelper.createMint(boss, BigInt(100_000e9), 9);

        await program.methods.initialize().accounts({boss}).rpc();
        await program.methods.initializeOffers().accounts({
            state: testHelper.statePda
        }).rpc();
    });

    test("Make a buy offer should succeed", async () => {
        // when
        await testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        });

        // then
        const [buyOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from("buy_offers")], ONREAPP_PROGRAM_ID);
        const buyOfferAccountData = await testHelper.program.account.buyOfferAccount.fetch(buyOfferAccountPda);

        expect(buyOfferAccountData.counter.toNumber()).toBe(1);

        const firstOffer = buyOfferAccountData.offers[0];
        expect(firstOffer.offerId.toNumber()).toBe(1);
        expect(firstOffer.tokenInMint.toString()).toBe(tokenInMint.toString());
        expect(firstOffer.tokenOutMint.toString()).toBe(tokenOutMint.toString());
    });

    test("Make multiple offers should succeed", async () => {
        // given
        const [buyOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from("buy_offers")], ONREAPP_PROGRAM_ID);
        const initialData = await testHelper.program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        const initialCounter = initialData.counter.toNumber();

        // when
        // make first offer
        const token1In = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const token1Out = testHelper.createMint(boss, BigInt(100_000e9), 9);

        await testHelper.makeBuyOffer({
            tokenInMint: token1In,
            tokenOutMint: token1Out
        });

        const token2In = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const token2Out = testHelper.createMint(boss, BigInt(100_000e9), 9);

        // make second offer
        await testHelper.makeBuyOffer({
            tokenInMint: token2In,
            tokenOutMint: token2Out
        });

        // then
        const buyOfferAccountData = await testHelper.program.account.buyOfferAccount.fetch(buyOfferAccountPda);

        expect(buyOfferAccountData.counter.toNumber()).toBe(initialCounter + 2);

        // Find offers by their auto-generated IDs
        const firstOffer = buyOfferAccountData.offers.find(offer =>
            offer.tokenInMint.toString() === token1In.toString() &&
            offer.offerId.toNumber() > initialCounter
        );
        expect(firstOffer).toBeDefined();
        expect(firstOffer!.offerId.toNumber()).toBe(initialCounter + 1);
        expect(firstOffer!.tokenOutMint.toString()).toBe(token1Out.toString());

        const secondOffer = buyOfferAccountData.offers.find(offer =>
            offer.tokenInMint.toString() === token2In.toString()
        );
        expect(secondOffer).toBeDefined();
        expect(secondOffer!.offerId.toNumber()).toBe(initialCounter + 2);
        expect(secondOffer!.tokenOutMint.toString()).toBe(token2Out.toString());
    });

    test("Make an offer with invalid token mints should fail", async () => {
        // when
        await expect(testHelper.makeBuyOffer({
            tokenInMint: new PublicKey(0),
            tokenOutMint: new PublicKey(0)
        })).rejects.toThrow();
    });

    test("Make more than max offers should fail", async () => {
        // Create MAX_BUY_OFFERS offers
        for (let i = 0; i < MAX_BUY_OFFERS; i++) {
            // Create unique mints for each offer to avoid duplicate transaction issues
            const uniqueTokenIn = testHelper.createMint(boss, BigInt(100_000e9), 9);
            const uniqueTokenOut = testHelper.createMint(boss, BigInt(100_000e9), 9);

            await testHelper.makeBuyOffer({
                tokenInMint: uniqueTokenIn,
                tokenOutMint: uniqueTokenOut
            });
        }

        // Verify array is full
        const [buyOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from("buy_offers")], ONREAPP_PROGRAM_ID);
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        const activeOffers = buyOfferAccount.offers.filter(offer => offer.offerId.toNumber() > 0).length;
        expect(activeOffers).toBe(MAX_BUY_OFFERS);

        // when - try to make one more offer (should fail)
        await expect(testHelper.makeBuyOffer({
            tokenInMint,
            tokenOutMint
        })).rejects.toThrow("Buy offer account is full, cannot create more offers");
    });
});