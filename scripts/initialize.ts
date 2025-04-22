import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import bs58 from "bs58";

import { initProgram } from "./script-commons";

const PROGRAM_ID = new PublicKey("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe");

const BOSS = new PublicKey("45YnzauhsBM8CpUz96Djf8UG5vqq2Dua62wuW9H3jaJ5"); // WARN: SQUAD MAIN ACCOUNT!!!

async function createMakeOfferOneTransaction() {
  const connection = new anchor.web3.Connection(
    process.env.SOL_MAINNET_RPC_URL || "",
  );
  const program = await initProgram();

  // Derive the state PDA
  const [statePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("state")],
    PROGRAM_ID,
  );

  try {
    const tx = await program.methods
      .initialize()
      .accountsPartial({
        state: statePda,
        boss: BOSS,
      })
      .transaction();

    tx.feePayer = BOSS;
    tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

    const serializedTx = tx.serialize({
      requireAllSignatures: false,
      verifySignatures: false,
    });

    const base58Tx = bs58.encode(serializedTx);
    console.log("Make Initialize Transaction (Base58):");
    console.log(base58Tx);

    return base58Tx;
  } catch (error) {
    console.error("Error creating transaction:", error);
    throw error;
  }
}

async function main() {
  try {
    await createMakeOfferOneTransaction();
  } catch (error) {
    console.error("Failed to create initialize transaction:", error);
  }
}

main();
