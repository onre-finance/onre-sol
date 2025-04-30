import { Connection, Keypair, Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
import fs from "fs";
import os from "os";
import path from "path";
import bs58 from "bs58";

// Function to load wallet from ~/.config/solana/id.json
function loadWalletKey(): Keypair {
    const home = os.homedir();
    const walletPath = path.join(home, ".config", "solana", "id.json");
    const walletKeypair = JSON.parse(fs.readFileSync(walletPath, "utf-8"));
    return Keypair.fromSecretKey(new Uint8Array(walletKeypair));
}

async function signAndSendTransaction(base58Transaction: string) {
    // Connect to Solana network using the same RPC URL as initialize.ts
    const connection = new Connection(process.env.SOL_MAINNET_RPC_URL || "", {
        commitment: "confirmed",
    });

    // Load your wallet
    const wallet = loadWalletKey();

    // Decode base58 transaction
    const serializedTransaction = bs58.decode(base58Transaction);

    // Deserialize the transaction
    const transaction = Transaction.from(serializedTransaction);

    // Sign the transaction
    transaction.partialSign(wallet);

    try {
        // Send and confirm transaction
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

// Usage example:
const base58Tx = process.argv[2];
if (!base58Tx) {
    console.error("Please provide the base58 transaction string as an argument");
    process.exit(1);
}

signAndSendTransaction(base58Tx).catch((error) => {
    console.error("Failed to send transaction:", error);
    process.exit(1);
});
