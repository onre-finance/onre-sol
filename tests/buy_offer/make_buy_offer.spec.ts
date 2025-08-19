import { PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

describe("make offer", () => {
    let testHelper: TestHelper;

    let tokenInMint: PublicKey;
    let tokenOutMint: PublicKey;

    let boss: PublicKey;

    beforeEach(async () => {
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
        // when
        // make first offer
        await testHelper.makeBuyOffer({
            offerId: new BN(1),
            tokenInMint,
            tokenOutMint,
        });

        const token2In = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const token2Out = testHelper.createMint(boss, BigInt(100_000e9), 9);

        // make second offer
        await testHelper.makeBuyOffer({
            offerId: new BN(2),
            tokenInMint: token2In,
            tokenOutMint: token2Out,
        });

        // then
        const [buyOfferAccountPda] = PublicKey.findProgramAddressSync([Buffer.from('buy_offers')], ONREAPP_PROGRAM_ID);
        const buyOfferAccountData = await testHelper.program.account.buyOfferAccount.fetch(buyOfferAccountPda);
        
        expect(buyOfferAccountData.count.toNumber()).toBe(2);

        const firstOffer = buyOfferAccountData.offers[0];
        expect(firstOffer.offerId.toNumber()).toBe(1);
        expect(firstOffer.tokenInMint.toString()).toBe(tokenInMint.toString());
        expect(firstOffer.tokenOutMint.toString()).toBe(tokenOutMint.toString());

        const secondOffer = buyOfferAccountData.offers[1];
        expect(secondOffer.offerId.toNumber()).toBe(2);
        expect(secondOffer.tokenInMint.toString()).toBe(token2In.toString());
        expect(secondOffer.tokenOutMint.toString()).toBe(token2Out.toString());
    });
});