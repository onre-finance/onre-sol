import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

const MAX_BUY_OFFERS = 10;

describe("Make buy offer", () => {
    let testHelper: TestHelper;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    let boss: PublicKey;

    beforeAll(async () => {
        const programInfo: AddedProgram = {
            programId: ONREAPP_PROGRAM_ID,
            name: "onreapp",
        };

        const workspace = process.cwd();
        const context = await startAnchor(workspace, [programInfo], []);

        const provider = new BankrunProvider(context);
        const program = new Program<Onreapp>(
            idl,
            provider,
        );

        testHelper = new TestHelper(context, program);

        boss = provider.wallet.publicKey;
        
        // Create mints
        tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        tokenOutMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
        
        await program.methods.initialize().accounts({ boss }).rpc();
        await program.methods.initializeOffers().accounts({ 
            state: testHelper.statePda 
        }).rpc();
    });

    test("Make a buy offer should succeed", async () => {
        // when
        await testHelper.makeBuyOffer({
            offerId: new BN(1),
            tokenInMint,
            tokenOutMint,
        });

        // then
        const [buyOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from('buy_offers')], ONREAPP_PROGRAM_ID);
        const buyOfferAccountData = await testHelper.program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        
        expect(buyOfferAccountData.count.toNumber()).toBe(1);
        
        const firstOffer = buyOfferAccountData.offers[0];
        expect(firstOffer.offerId.toNumber()).toBe(1);
        expect(firstOffer.tokenInMint.toString()).toBe(tokenInMint.toString());
        expect(firstOffer.tokenOutMint.toString()).toBe(tokenOutMint.toString());
    });

    test("Make multiple offers should succeed", async () => {
        // given
        const [buyOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from('buy_offers')], ONREAPP_PROGRAM_ID);
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        const offerCount = buyOfferAccount.count.toNumber();

        // when
        // make one offer
        await testHelper.makeBuyOffer({
            offerId: new BN(4321),
            tokenInMint,
            tokenOutMint,
        });

        const token2In = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const token2Out = testHelper.createMint(boss, BigInt(100_000e9), 9);

        // make another offer
        await testHelper.makeBuyOffer({
            offerId: new BN(1234),
            tokenInMint: token2In,
            tokenOutMint: token2Out,
        });

        // then
        const buyOfferAccountData = await testHelper.program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        
        expect(buyOfferAccountData.count.toNumber()).toBe(offerCount + 2);

        const firstOffer = buyOfferAccountData.offers.find(offer => offer.offerId.toNumber() === 4321);
        expect(firstOffer.offerId.toNumber()).toBe(4321);
        expect(firstOffer.tokenInMint.toString()).toBe(tokenInMint.toString());
        expect(firstOffer.tokenOutMint.toString()).toBe(tokenOutMint.toString());

        const secondOffer = buyOfferAccountData.offers.find(offer => offer.offerId.toNumber() === 1234);
        expect(secondOffer.offerId.toNumber()).toBe(1234);
        expect(secondOffer.tokenInMint.toString()).toBe(token2In.toString());
        expect(secondOffer.tokenOutMint.toString()).toBe(token2Out.toString());
    });

    test("Make an offer with invalid token mints should fail", async () => {
        // when
        await expect(testHelper.makeBuyOffer({
            offerId: new BN(1),
            tokenInMint: new PublicKey(0),
            tokenOutMint: new PublicKey(0),
        })).rejects.toThrow();
    });

    test("Make more than max offers should fail", async () => {
        // given
        const [buyOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from('buy_offers')], ONREAPP_PROGRAM_ID);
        const buyOfferAccount = await testHelper.program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        const offerCount = buyOfferAccount.count.toNumber();

        for (let i = offerCount; i < MAX_BUY_OFFERS; i++) {
            await testHelper.makeBuyOffer({
                offerId: new BN(i),
                tokenInMint,
                tokenOutMint,
            });
        }

        // when
        await expect(testHelper.makeBuyOffer({
            offerId: new BN(1),
            tokenInMint,
            tokenOutMint,
        })).rejects.toThrow();
    });
});