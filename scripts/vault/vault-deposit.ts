import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// Configuration - UPDATE THESE
const TOKEN_MINT = new PublicKey("FsSJSYJKLdyxtsT25DoyLS1j2asxzBkTVuX8vtojyWob"); // USDC Mint Address
const AMOUNT = 1_000_000_000; // 1000 tokens with 6 decimals for USDC

async function createVaultDepositTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating vault deposit transaction...");
    console.log("Token mint:", TOKEN_MINT.toBase58());
    console.log("Amount:", AMOUNT);

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        const tx = await helper.buildOfferVaultDepositTransaction({
            amount: AMOUNT,
            tokenMint: TOKEN_MINT
        });

        return helper.printTransaction(tx, "Vault Deposit Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createVaultDepositTransaction();
    } catch (error) {
        console.error("Failed to create vault deposit transaction:", error);
    }
}

await main();