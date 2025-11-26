import { ScriptHelper, TOKEN_OUT_MINT } from "../utils/script-helper";

// Configuration - UPDATE THESE
const TOKEN_MINT = TOKEN_OUT_MINT; // Using default TOKEN_OUT_MINT from script-helper
const AMOUNT = 100_000_000_000; // 100 tokens with 9 decimals for USDC

async function createVaultDepositTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating vault deposit transaction...");
    console.log("Token mint:", TOKEN_MINT.toBase58());
    console.log("Amount:", AMOUNT);

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    const bossTokenAccounts = await helper.connection.getTokenAccountsByOwner(boss, { mint: TOKEN_MINT });
    if (bossTokenAccounts.value.length > 0) {
        const bossBalance = await helper.connection.getTokenAccountBalance(bossTokenAccounts.value[0].pubkey);
        console.log("Boss token balance:", bossBalance.value.uiAmountString);
    } else {
        console.log("Boss token balance: No token account found");
    }

    try {
        const ix = await helper.buildOfferVaultDepositIx({
            amount: AMOUNT,
            tokenMint: TOKEN_MINT
        });

        const tx = await helper.prepareTransaction(ix);

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
