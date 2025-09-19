import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import bs58 from "bs58";

import { initProgram, PROGRAM_ID, RPC_URL } from "./script-commons";

// PROD
// const BOSS = new PublicKey("45YnzauhsBM8CpUz96Djf8UG5vqq2Dua62wuW9H3jaJ5"); // WARN: SQUAD MAIN ACCOUNT!!!

// TEST & local
const BOSS = new PublicKey("7rzEKejyAXJXMkGfRhMV9Vg1k7tFznBBEFu3sfLNz8LC"); // DEV Squad

async function createKillSwitchDisableTransaction() {
    const connection = new anchor.web3.Connection(RPC_URL);
    const program = await initProgram();

    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from("state")], PROGRAM_ID);
    const [killSwitchStatePda] = PublicKey.findProgramAddressSync([Buffer.from("kill_switch")], PROGRAM_ID);
    const [adminStatePda] = PublicKey.findProgramAddressSync([Buffer.from("admin_state")], PROGRAM_ID);

    try {
        const tx = await program.methods
            .killSwitch(false) // enable = false
            .accountsPartial({
                killSwitchState: killSwitchStatePda,
                adminState: adminStatePda,
                state: statePda,
                signer: BOSS,
            })
            .transaction();

        tx.feePayer = BOSS;
        tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

        const serializedTx = tx.serialize({
            requireAllSignatures: false,
            verifySignatures: false,
        });

        const base58Tx = bs58.encode(serializedTx);
        console.log("Kill Switch Disable Transaction (Base58):");
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error("Error creating kill switch disable transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createKillSwitchDisableTransaction();
    } catch (error) {
        console.error("Failed to create kill switch disable transaction:", error);
    }
}

main();