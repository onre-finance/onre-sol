import { Keypair, PublicKey } from "@solana/web3.js";
import { ONREAPP_PROGRAM_ID, TestHelper } from "../test_helper";
import { AddedProgram, startAnchor } from "solana-bankrun";
import { Onreapp } from "../../target/types/onreapp";
import { BankrunProvider } from "anchor-bankrun";
import { BN, Program } from "@coral-xyz/anchor";
import idl from "../../target/idl/onreapp.json";

describe("Close dual redemption offer", () => {
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

  test("Close dual redemption offer should succeed", async () => {
    // given - create a dual redemption offer first
    const tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    const startTime = new BN(Math.floor(Date.now() / 1000));
    const endTime = new BN(startTime.toNumber() + 3600);
    const price1 = new BN(1500000000);
    const price2 = new BN(2000000000);
    const ratioBasisPoints = 8000;

    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime, endTime, price1, price2, ratioBasisPoints)
      .accounts({
        tokenInMint,
        tokenOutMint1,
        tokenOutMint2,
        state: testHelper.statePda,
      })
      .rpc();

    // Get the created offer ID
    const [dualRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("dual_redemption_offers")],
      ONREAPP_PROGRAM_ID
    );
    const beforeCloseData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    const createdOffer = beforeCloseData.offers.find(offer => 
      offer.tokenInMint.toString() === tokenInMint.toString()
    );
    expect(createdOffer).toBeDefined();
    const offerId = createdOffer!.offerId.toNumber();

    // when - close the offer
    await testHelper.program.methods
      .closeDualRedemptionOffer(new BN(offerId))
      .accounts({
        state: testHelper.statePda,
      })
      .rpc();

    // then - verify offer is cleared
    const afterCloseData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    
    // Count active offers before and after (active = offerId != 0)
    const activeOffersBefore = beforeCloseData.offers.filter(offer => offer.offerId.toNumber() !== 0).length;
    const activeOffersAfter = afterCloseData.offers.filter(offer => offer.offerId.toNumber() !== 0).length;
    
    // Should be one less active offer
    expect(activeOffersAfter).toBe(activeOffersBefore - 1);
    
    // Cannot find the specific offer by ID anymore (because ID is now 0)
    const closedOfferSearch = afterCloseData.offers.find(offer => offer.offerId.toNumber() === offerId);
    expect(closedOfferSearch).toBeUndefined();
  });

  test("Close multiple dual redemption offers should succeed", async () => {
    // given - create two dual redemption offers
    const token1InMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const token1OutMint1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const token1OutMint2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    const token2InMint = testHelper.createMint(boss, BigInt(100_000e9), 18);
    const token2OutMint1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const token2OutMint2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    const startTime = new BN(Math.floor(Date.now() / 1000));
    const endTime = new BN(startTime.toNumber() + 3600);
    const price1 = new BN(1000000000);
    const price2 = new BN(500000000);

    // Create first offer
    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime, endTime, price1, price2, 7000)
      .accounts({
        tokenInMint: token1InMint,
        tokenOutMint1: token1OutMint1,
        tokenOutMint2: token1OutMint2,
        state: testHelper.statePda,
      })
      .rpc();

    // Create second offer
    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime, endTime, price1, price2, 9000)
      .accounts({
        tokenInMint: token2InMint,
        tokenOutMint1: token2OutMint1,
        tokenOutMint2: token2OutMint2,
        state: testHelper.statePda,
      })
      .rpc();

    // Get offer IDs
    const [dualRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("dual_redemption_offers")],
      ONREAPP_PROGRAM_ID
    );
    const beforeCloseData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    
    const offer1 = beforeCloseData.offers.find(offer => 
      offer.tokenInMint.toString() === token1InMint.toString()
    );
    const offer2 = beforeCloseData.offers.find(offer => 
      offer.tokenInMint.toString() === token2InMint.toString()
    );
    
    expect(offer1).toBeDefined();
    expect(offer2).toBeDefined();
    
    const offerId1 = offer1!.offerId.toNumber();
    const offerId2 = offer2!.offerId.toNumber();

    // when - close both offers
    await testHelper.program.methods
      .closeDualRedemptionOffer(new BN(offerId1))
      .accounts({
        state: testHelper.statePda,
      })
      .rpc();

    await testHelper.program.methods
      .closeDualRedemptionOffer(new BN(offerId2))
      .accounts({
        state: testHelper.statePda,
      })
      .rpc();

    // then - verify both offers are cleared
    const afterCloseData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    
    const closedOffer1 = afterCloseData.offers.find(offer => 
      offer.offerId.toNumber() === offerId1
    );
    const closedOffer2 = afterCloseData.offers.find(offer => 
      offer.offerId.toNumber() === offerId2
    );

    // Both should be cleared (offerId = 0)
    expect(closedOffer1?.offerId.toNumber()).toBeUndefined();
    expect(closedOffer2?.offerId.toNumber()).toBeUndefined();
  });

  test("Close dual redemption offer with invalid offer ID should fail", async () => {
    // when/then - try to close non-existent offer
    await expect(
      testHelper.program.methods
        .closeDualRedemptionOffer(new BN(99999))
        .accounts({
          state: testHelper.statePda,
        })
        .rpc()
    ).rejects.toThrow("Offer not found");
  });

  test("Close dual redemption offer with zero offer ID should fail", async () => {
    // when/then - try to close with zero ID
    await expect(
      testHelper.program.methods
        .closeDualRedemptionOffer(new BN(0))
        .accounts({
          state: testHelper.statePda,
        })
        .rpc()
    ).rejects.toThrow("Offer not found");
  });

  test("Close dual redemption offer should fail when not called by boss", async () => {
    // given - create a dual redemption offer first
    const tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    const startTime = new BN(Math.floor(Date.now() / 1000));
    const endTime = new BN(startTime.toNumber() + 3600);
    const price1 = new BN(1000000000);
    const price2 = new BN(500000000);

    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime, endTime, price1, price2, 8000)
      .accounts({
        tokenInMint,
        tokenOutMint1,
        tokenOutMint2,
        state: testHelper.statePda,
      })
      .rpc();

    // Get the offer ID
    const [dualRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("dual_redemption_offers")],
      ONREAPP_PROGRAM_ID
    );
    const data = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    const offer = data.offers.find(offer => 
      offer.tokenInMint.toString() === tokenInMint.toString()
    );
    const offerId = offer!.offerId.toNumber();

    // when/then - try to close with different signer
    const fakeUser = Keypair.generate();
    
    await expect(
      testHelper.program.methods
        .closeDualRedemptionOffer(new BN(offerId))
        .accountsPartial({
          state: testHelper.statePda,
          boss: fakeUser.publicKey,
        })
        .signers([fakeUser])
        .rpc()
    ).rejects.toThrow();
  });

  test("Close already closed dual redemption offer should fail", async () => {
    // given - create and close a dual redemption offer
    const tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    const startTime = new BN(Math.floor(Date.now() / 1000));
    const endTime = new BN(startTime.toNumber() + 3600);
    const price1 = new BN(1000000000);
    const price2 = new BN(500000000);

    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime, endTime, price1, price2, 8000)
      .accounts({
        tokenInMint,
        tokenOutMint1,
        tokenOutMint2,
        state: testHelper.statePda,
      })
      .rpc();

    // Get the offer ID
    const [dualRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("dual_redemption_offers")],
      ONREAPP_PROGRAM_ID
    );
    const data = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    const offer = data.offers.find(offer => 
      offer.tokenInMint.toString() === tokenInMint.toString()
    );
    const offerId = offer!.offerId.toNumber();

    // Close the offer once
    await testHelper.program.methods
      .closeDualRedemptionOffer(new BN(offerId))
      .accounts({
        state: testHelper.statePda,
      })
      .rpc();

    // when/then - try to close the same offer again (should fail)
    // Add small delay to ensure unique transaction
    await new Promise(resolve => setTimeout(resolve, 10));
    
    await expect(
      testHelper.program.methods
        .closeDualRedemptionOffer(new BN(offerId))
        .accounts({
          state: testHelper.statePda,
        })
        .rpc()
    ).rejects.toThrow("Offer not found");
  });

  test("Verify dual redemption offer counter remains unchanged after closing", async () => {
    // given - get initial counter
    const [dualRedemptionOfferAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("dual_redemption_offers")],
      ONREAPP_PROGRAM_ID
    );
    const initialData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    const initialCounter = initialData.counter.toNumber();

    // Create an offer
    const tokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const tokenOutMint2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    const startTime = new BN(Math.floor(Date.now() / 1000));
    const endTime = new BN(startTime.toNumber() + 3600);
    const price1 = new BN(1000000000);
    const price2 = new BN(500000000);

    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime, endTime, price1, price2, 8000)
      .accounts({
        tokenInMint,
        tokenOutMint1,
        tokenOutMint2,
        state: testHelper.statePda,
      })
      .rpc();

    // Get the offer ID and close it
    const afterCreateData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    const offer = afterCreateData.offers.find(offer => 
      offer.tokenInMint.toString() === tokenInMint.toString()
    );
    const offerId = offer!.offerId.toNumber();

    await testHelper.program.methods
      .closeDualRedemptionOffer(new BN(offerId))
      .accounts({
        state: testHelper.statePda,
      })
      .rpc();

    // then - verify counter is still incremented (not decremented)
    const finalData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    expect(finalData.counter.toNumber()).toBe(initialCounter + 1);

    // Verify we can still use the same array slot for a new offer
    const newTokenInMint = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const newTokenOutMint1 = testHelper.createMint(boss, BigInt(100_000e9), 9);
    const newTokenOutMint2 = testHelper.createMint(boss, BigInt(100_000e9), 6);

    await testHelper.program.methods
      .makeDualRedemptionOffer(startTime, endTime, price1, price2, 5000)
      .accounts({
        tokenInMint: newTokenInMint,
        tokenOutMint1: newTokenOutMint1,
        tokenOutMint2: newTokenOutMint2,
        state: testHelper.statePda,
      })
      .rpc();

    const afterNewOfferData = await testHelper.program.account.dualRedemptionOfferAccount.fetch(dualRedemptionOfferAccountPda);
    expect(afterNewOfferData.counter.toNumber()).toBe(initialCounter + 2);
  });
});