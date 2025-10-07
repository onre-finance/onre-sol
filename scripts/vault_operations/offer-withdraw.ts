import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// Configuration - UPDATE THESE
const TOKEN_MINT = new PublicKey("FsSJSYJKLdyxtsT25DoyLS1j2asxzBkTVuX8vtojyWob"); // USDC Mint Address
const AMOUNT = 100_000_000; // 100 tokens with 6 decimals for USDC

async function createVaultWithdrawTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating vault withdraw transaction...");
    console.log("Token mint:", TOKEN_MINT.toBase58());
    console.log("Amount:", AMOUNT);

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        const ix = await helper.buildOfferVaultWithdrawIx({
            amount: AMOUNT,
            tokenMint: TOKEN_MINT
        });

        const tx = await helper.prepareTransaction(ix);

        return helper.printTransaction(tx, "Vault Withdraw Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createVaultWithdrawTransaction();
    } catch (error) {
        console.error("Failed to create vault withdraw transaction:", error);
    }
}

await main();