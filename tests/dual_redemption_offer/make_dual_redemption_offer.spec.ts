import { Keypair, PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

const MAX_DUAL_REDEMPTION_OFFERS = 50;

describe("Make dual redemption offer", () => {
  let testHelper: TestHelper;
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

    // Initialize program state and offers
    await program.methods.initialize().accounts({ boss }).rpc();
    await program.methods.initializeOffers().accounts({
      state: testHelper.statePda
    }).rpc();
  });

  test("Make a dual redemption offer should succeed", async () => {
    // given
    const tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    const startTime = new BN(Math.floor(Date.now() / 1000) - 60); // 1 minute ago
    const endTime = new BN(startTime.toNumber() + 3600); // 1 hour from start
    const price1 = new BN(1500000000); // 1.5 with 9 decimals
    const price2 = new BN(2000000000); // 2.0 with 9 decimals
    const ratioBasisPoints = new BN(8000); // 80% for token_out_1, 20% for token_out_2

    // when
    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime, endTime, price1, price2, ratioBasisPoints)
      .accounts({
        tokenInMint,
        tokenOutMint1,
        tokenOutMint2,
        state: testHelper.statePda,
      })
      .rpc();

    // then
    const [dualRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("dual_redemption_offers")],
      ONREAPP_PROGRAM_ID
    );
    const dualRedemptionOfferAccountData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(
      dualRedemptionOfferAccountPda
    );

    expect(dualRedemptionOfferAccountData.counter.toNumber()).toBe(1);

    const firstOffer = dualRedemptionOfferAccountData.offers[0];
    expect(firstOffer.offerId.toNumber()).toBe(1);
    expect(firstOffer.tokenInMint.toString()).toBe(tokenInMint.toString());
    expect(firstOffer.tokenOutMint1.toString()).toBe(tokenOutMint1.toString());
    expect(firstOffer.tokenOutMint2.toString()).toBe(tokenOutMint2.toString());
    expect(firstOffer.price1.toNumber()).toBe(price1.toNumber());
    expect(firstOffer.price2.toNumber()).toBe(price2.toNumber());
    expect(firstOffer.ratioBasisPoints.toNumber()).toBe(ratioBasisPoints.toNumber());
    expect(firstOffer.startTime.toNumber()).toBe(startTime.toNumber());
    expect(firstOffer.endTime.toNumber()).toBe(endTime.toNumber());
  });

  test("Make multiple dual redemption offers should succeed", async () => {
    // given
    const [dualRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("dual_redemption_offers")],
      ONREAPP_PROGRAM_ID
    );
    const initialData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    const initialCounter = initialData.counter.toNumber();

    // when - make first offer
    const token1In = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const token1Out1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const token1Out2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    const startTime1 = new BN(Math.floor(Date.now() / 1000));
    const endTime1 = new BN(startTime1.toNumber() + 3600);
    const price1_1 = new BN(1000000000); // 1.0
    const price1_2 = new BN(500000000); // 0.5
    const ratio1 = new BN(7000); // 70/30 split

    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime1, endTime1, price1_1, price1_2, ratio1)
      .accounts({
        tokenInMint: token1In,
        tokenOutMint1: token1Out1,
        tokenOutMint2: token1Out2,
        state: testHelper.statePda,
      })
      .rpc();

    // make second offer
    const token2In = testHelper.createMint(boss, BigInt(100_000e9), 18);
    const token2Out1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const token2Out2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    const startTime2 = new BN(Math.floor(Date.now() / 1000));
    const endTime2 = new BN(startTime2.toNumber() + 7200);
    const price2_1 = new BN(2500000000); // 2.5
    const price2_2 = new BN(1200000000); // 1.2
    const ratio2 = new BN(9000); // 90/10 split

    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime2, endTime2, price2_1, price2_2, ratio2)
      .accounts({
        tokenInMint: token2In,
        tokenOutMint1: token2Out1,
        tokenOutMint2: token2Out2,
        state: testHelper.statePda,
      })
      .rpc();

    // then
    const dualRedemptionOfferAccountData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);

    expect(dualRedemptionOfferAccountData.counter.toNumber()).toBe(initialCounter + 2);

    // Find offers by their properties
    const firstOffer = dualRedemptionOfferAccountData.offers.find(offer =>
      offer.tokenInMint.toString() === token1In.toString() &&
      offer.offerId.toNumber() > initialCounter
    );
    expect(firstOffer).toBeDefined();
    expect(firstOffer!.offerId.toNumber()).toBe(initialCounter + 1);
    expect(firstOffer!.tokenOutMint1.toString()).toBe(token1Out1.toString());
    expect(firstOffer!.tokenOutMint2.toString()).toBe(token1Out2.toString());
    expect(firstOffer!.ratioBasisPoints.toNumber()).toBe(ratio1.toNumber());

    const secondOffer = dualRedemptionOfferAccountData.offers.find(offer =>
      offer.tokenInMint.toString() === token2In.toString()
    );
    expect(secondOffer).toBeDefined();
    expect(secondOffer!.offerId.toNumber()).toBe(initialCounter + 2);
    expect(secondOffer!.tokenOutMint1.toString()).toBe(token2Out1.toString());
    expect(secondOffer!.tokenOutMint2.toString()).toBe(token2Out2.toString());
    expect(secondOffer!.ratioBasisPoints.toNumber()).toBe(ratio2.toNumber());
  });

  test("Make dual redemption offer with invalid ratio should fail", async () => {
    // given
    const tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    const startTime = new BN(Math.floor(Date.now() / 1000));
    const endTime = new BN(startTime.toNumber() + 3600);
    const price1 = new BN(1000000000);
    const price2 = new BN(500000000);
    const invalidRatio = new BN(10001); // > 10000 (100%)

    // when/then
    await expect(
      testHelper.program.methods
        .makeDualRedemptionOffer(startTime, endTime, price1, price2, invalidRatio)
        .accounts({
          tokenInMint,
          tokenOutMint1,
          tokenOutMint2,
          state: testHelper.statePda,
        })
        .rpc()
    ).rejects.toThrow("Invalid ratio");
  });

  test("Make dual redemption offer with edge case ratios should succeed", async () => {
    // Test 0% ratio (all goes to token_out_2)
    const tokenInMint1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint1_1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint1_2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    const startTime = new BN(Math.floor(Date.now() / 1000));
    const endTime = new BN(startTime.toNumber() + 3600);
    const price1 = new BN(1000000000);
    const price2 = new BN(500000000);

    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime, endTime, price1, price2, new BN(0)) // 0% for token_out_1
      .accounts({
        tokenInMint: tokenInMint1,
        tokenOutMint1: tokenOutMint1_1,
        tokenOutMint2: tokenOutMint1_2,
        state: testHelper.statePda,
      })
      .rpc();

    // Test 100% ratio (all goes to token_out_1)
    const tokenInMint2 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint2_1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint2_2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime, endTime, price1, price2, new BN(10000)) // 100% for token_out_1
      .accounts({
        tokenInMint: tokenInMint2,
        tokenOutMint1: tokenOutMint2_1,
        tokenOutMint2: tokenOutMint2_2,
        state: testHelper.statePda,
      })
      .rpc();

    // Verify both offers were created
    const [dualRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("dual_redemption_offers")],
      ONREAPP_PROGRAM_ID
    );
    const accountData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);

    const offer0 = accountData.offers.find(offer => offer.tokenInMint.toString() === tokenInMint1.toString());
    expect(offer0!.ratioBasisPoints.toNumber()).toBe(0);

    const offer100 = accountData.offers.find(offer => offer.tokenInMint.toString() === tokenInMint2.toString());
    expect(offer100!.ratioBasisPoints.toNumber()).toBe(10000);
  });

  test("Make dual redemption offer should fail when not called by boss", async () => {
    // given
    const startTime = new BN(Math.floor(Date.now() / 1000));
    const endTime = new BN(startTime.toNumber() + 3600);
    const price1 = new BN(1000000000);
    const price2 = new BN(500000000);

    const uniqueTokenIn = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const uniqueTokenOut1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const uniqueTokenOut2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    // when/then - try to create with different signer
    const fakeUser = Keypair.generate();
    
    await expect(
      testHelper.program.methods
        .makeDualRedemptionOffer(startTime, endTime, price1, price2, new BN(8000))
        .accountsPartial({
          tokenInMint: uniqueTokenIn,
          tokenOutMint1: uniqueTokenOut1,
          tokenOutMint2: uniqueTokenOut2,
          state: testHelper.statePda,
          boss: fakeUser.publicKey,
        })
        .signers([fakeUser])
        .rpc()
    ).rejects.toThrow();
  });

  test("Make more than max dual redemption offers should fail", async () => {
    // given - check how many offers already exist
    const [dualRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("dual_redemption_offers")],
      ONREAPP_PROGRAM_ID
    );
    let dualRedemptionOfferAccount = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    const existingOffers = dualRedemptionOfferAccount.offers.filter(offer => offer.offerId.toNumber() > 0).length;

    console.log(`Existing dual redemption offers: ${existingOffers}`);

    // Fill up remaining slots
    const offersToMake = MAX_DUAL_REDEMPTION_OFFERS - existingOffers;
    console.log(`Need to make ${offersToMake} more dual redemption offers`);

    const startTime = new BN(Math.floor(Date.now() / 1000));
    const endTime = new BN(startTime.toNumber() + 3600);
    const price1 = new BN(1000000000);
    const price2 = new BN(500000000);

    for (let i = 0; i < offersToMake; i++) {
      console.log(`Making dual redemption offer ${i + 1}/${offersToMake}`);
      try {
        // Create unique mints for each offer to avoid duplicate transaction issues
        const uniqueTokenIn = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const uniqueTokenOut1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
        const uniqueTokenOut2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

        await testHelper.program.methods
          .makeDualRedemptionOffer(startTime, endTime, price1, price2, new BN(8000))
          .accounts({
            tokenInMint: uniqueTokenIn,
            tokenOutMint1: uniqueTokenOut1,
            tokenOutMint2: uniqueTokenOut2,
            state: testHelper.statePda,
          })
          .rpc();
      } catch (error) {
        console.log(`Error making dual redemption offer ${i + 1}:`, error);
        throw error;
      }
    }

    // Verify array is full
    dualRedemptionOfferAccount = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    const activeOffers = dualRedemptionOfferAccount.offers.filter(offer => offer.offerId.toNumber() > 0).length;
    console.log(`Final active dual redemption offers: ${activeOffers}`);
    expect(activeOffers).toBe(MAX_DUAL_REDEMPTION_OFFERS);

    // when - try to make one more offer (should fail)
    console.log("Attempting to make one more dual redemption offer (should fail)");
    const finalTokenIn = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const finalTokenOut1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const finalTokenOut2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    await expect(
      testHelper.program.methods
        .makeDualRedemptionOffer(startTime, endTime, price1, price2, new BN(8000))
        .accounts({
          tokenInMint: finalTokenIn,
          tokenOutMint1: finalTokenOut1,
          tokenOutMint2: finalTokenOut2,
          state: testHelper.statePda,
        })
        .rpc()
    ).rejects.toThrow("Dual redemption offer account is full");
  });
});