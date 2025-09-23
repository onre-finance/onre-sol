import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import bs58 from "bs58";

import { initProgram, PROGRAM_ID, RPC_URL } from "./script-commons";

// PROD
// const BOSS = new PublicKey("45YnzauhsBM8CpUz96Djf8UG5vqq2Dua62wuW9H3jaJ5"); // WARN: SQUAD MAIN ACCOUNT!!!

// TEST & local
const BOSS = new PublicKey("7rzEKejyAXJXMkGfRhMV9Vg1k7tFznBBEFu3sfLNz8LC"); // DEV Squad

// Admin to remove - UPDATE THIS
const ADMIN_TO_REMOVE = new PublicKey("REPLACE_WITH_ADMIN_PUBKEY");

async function createRemoveAdminTransaction(): Promise<string> {
    const connection = new anchor.web3.Connection(RPC_URL);
    const program = await initProgram();

    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from("state")], PROGRAM_ID);

    try {
        const tx = await program.methods
            .removeAdmin(ADMIN_TO_REMOVE)
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
        console.log(`Remove Admin Transaction (Base58) - Removing: ${ADMIN_TO_REMOVE.toString()}:`);
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error("Error creating remove admin transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createRemoveAdminTransaction();
    } catch (error) {
        console.error("Failed to create remove admin transaction:", error);
    }
}

main();