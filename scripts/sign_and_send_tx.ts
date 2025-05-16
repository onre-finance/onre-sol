import { Connection, Keypair, Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
import fs from "fs";
import os from "os";
import path from "path";
import bs58 from "bs58";

import { RPC_URL } from "./script-commons";

// Function to load wallet from ~/.config/solana/id.json
function loadWalletKey(): Keypair {
    const home = os.homedir();
    const walletPath = path.join(home, ".config", "solana", "id.json");
    const walletKeypair = JSON.parse(fs.readFileSync(walletPath, "utf-8"));
    return Keypair.fromSecretKey(new Uint8Array(walletKeypair));
}

async function signAndSendTransaction(base58Transaction: string) {
    const connection = new Connection(RPC_URL, {
        commitment: "confirmed",
    });

    const wallet = loadWalletKey();
    const transaction = Transaction.from(bs58.decode(base58Transaction));

    transaction.partialSign(wallet);

    try {
        const signature = await sendAndConfirmTransaction(connection, transaction, [wallet], {
            skipPreflight: false,
            commitment: "confirmed",
            preflightCommitment: "confirmed",
        });

        console.log("Transaction sent successfully!");
        console.log("Signature:", signature);
        return signature;
    } catch (error) {
        console.error("Error sending transaction:", error);
        throw error;
    }
}

const base58Tx = process.argv[2];
if (!base58Tx) {
    console.error("Please provide the base58 transaction string as an argument");
    process.exit(1);
}

signAndSendTransaction(base58Tx).catch((error) => {
    console.error("Failed to send transaction:", error);
    process.exit(1);
});
