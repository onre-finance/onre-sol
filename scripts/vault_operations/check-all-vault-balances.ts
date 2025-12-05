import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { ScriptHelper, USDC_MINT, ONYC_MINT } from "../utils/script-helper";

// Configuration - UPDATE THESE
const TOKENS_TO_CHECK = [
    { mint: USDC_MINT, name: "USDC", program: TOKEN_PROGRAM_ID },
    { mint: ONYC_MINT, name: "ONyc", program: TOKEN_PROGRAM_ID },
    // Add more tokens as needed
];

interface VaultBalance {
    vaultType: string;
    tokenName: string;
    mint: string;
    vaultAccount: string;
    exists: boolean;
    balance?: string;
    rawBalance?: string;
    decimals?: number;
}

async function checkVaultBalance(
    helper: ScriptHelper,
    tokenMint: PublicKey,
    tokenProgram: PublicKey,
    vaultAuthority: PublicKey,
    tokenName: string,
    vaultType: string
): Promise<VaultBalance> {
    const result: VaultBalance = {
        vaultType,
        tokenName,
        mint: tokenMint.toBase58(),
        vaultAccount: "",
        exists: false,
    };

    try {
        // Get the associated token account address
        const vaultTokenAccount = getAssociatedTokenAddressSync(
            tokenMint,
            vaultAuthority,
            true, // allowOwnerOffCurve
            tokenProgram
        );
        result.vaultAccount = vaultTokenAccount.toBase58();

        // Fetch the token account
        const accountInfo = await helper.connection.getAccountInfo(vaultTokenAccount);

        if (!accountInfo) {
            return result;
        }

        result.exists = true;

        // Parse token account data
        const tokenAccount = await helper.connection.getTokenAccountBalance(vaultTokenAccount);
        result.rawBalance = tokenAccount.value.amount;
        result.balance = tokenAccount.value.uiAmountString || "0";
        result.decimals = tokenAccount.value.decimals;

        return result;
    } catch (error) {
        console.error(`Error checking ${vaultType} vault for ${tokenName}:`, error);
        return result;
    }
}

async function checkAllVaultBalances() {
    const helper = await ScriptHelper.create();

    console.log("=".repeat(100));
    console.log(" ".repeat(35) + "VAULT BALANCE REPORT");
    console.log("=".repeat(100));
    console.log(`Program ID: ${helper.program.programId.toBase58()}`);
    console.log(`Network: ${helper.connection.rpcEndpoint}`);
    console.log("=".repeat(100));

    // Get vault authorities
    const offerVaultAuthority = helper.pdas.offerVaultAuthorityPda;
    const permissionlessVaultAuthority = helper.pdas.permissionlessVaultAuthorityPda;
    const redemptionVaultAuthority = PublicKey.findProgramAddressSync(
        [Buffer.from("redemption_offer_vault_authority")],
        helper.program.programId
    )[0];

    console.log("\nVault Authorities:");
    console.log("  Offer Vault Authority:", offerVaultAuthority.toBase58());
    console.log("  Permissionless Vault Authority:", permissionlessVaultAuthority.toBase58());
    console.log("  Redemption Vault Authority:", redemptionVaultAuthority.toBase58());
    console.log("");

    const results: VaultBalance[] = [];

    // Check each token in each vault
    for (const token of TOKENS_TO_CHECK) {
        console.log(`\nChecking ${token.name} (${token.mint.toBase58()})...`);

        // Check offer vault
        const offerVault = await checkVaultBalance(
            helper,
            token.mint,
            token.program,
            offerVaultAuthority,
            token.name,
            "Offer Vault"
        );
        results.push(offerVault);

        // Check permissionless vault
        const permissionlessVault = await checkVaultBalance(
            helper,
            token.mint,
            token.program,
            permissionlessVaultAuthority,
            token.name,
            "Permissionless Vault"
        );
        results.push(permissionlessVault);

        // Check redemption vault
        const redemptionVault = await checkVaultBalance(
            helper,
            token.mint,
            token.program,
            redemptionVaultAuthority,
            token.name,
            "Redemption Vault"
        );
        results.push(redemptionVault);
    }

    // Print summary table
    console.log("\n" + "=".repeat(100));
    console.log(" ".repeat(40) + "SUMMARY TABLE");
    console.log("=".repeat(100));
    console.log(
        "Vault Type".padEnd(25) +
        "Token".padEnd(10) +
        "Exists".padEnd(10) +
        "Balance".padEnd(20) +
        "Decimals"
    );
    console.log("-".repeat(100));

    for (const result of results) {
        const existsStr = result.exists ? "✓" : "✗";
        const balanceStr = result.exists ? result.balance || "0" : "N/A";
        const decimalsStr = result.exists ? result.decimals?.toString() || "?" : "N/A";

        console.log(
            result.vaultType.padEnd(25) +
            result.tokenName.padEnd(10) +
            existsStr.padEnd(10) +
            balanceStr.padEnd(20) +
            decimalsStr
        );
    }

    console.log("=".repeat(100));

    // Print detailed information for accounts with balance
    const accountsWithBalance = results.filter(r => r.exists && r.rawBalance !== "0");
    if (accountsWithBalance.length > 0) {
        console.log("\n" + "=".repeat(100));
        console.log(" ".repeat(30) + "ACCOUNTS WITH NON-ZERO BALANCE");
        console.log("=".repeat(100));

        for (const result of accountsWithBalance) {
            console.log(`\n${result.vaultType} - ${result.tokenName}:`);
            console.log(`  Account: ${result.vaultAccount}`);
            console.log(`  Balance: ${result.balance} (${result.rawBalance} raw)`);
            console.log(`  Decimals: ${result.decimals}`);
        }
        console.log("\n" + "=".repeat(100));
    }

    // Print warning for missing accounts
    const missingAccounts = results.filter(r => !r.exists);
    if (missingAccounts.length > 0) {
        console.log("\n" + "=".repeat(100));
        console.log(" ".repeat(35) + "MISSING ACCOUNTS");
        console.log("=".repeat(100));
        console.log("The following vault token accounts have not been initialized:\n");

        for (const result of missingAccounts) {
            console.log(`  ⚠️  ${result.vaultType} - ${result.tokenName}`);
            console.log(`     Expected at: ${result.vaultAccount}`);
        }
        console.log("\n" + "=".repeat(100));
    }
}

async function main() {
    try {
        await checkAllVaultBalances();
    } catch (error) {
        console.error("\n❌ Failed to check vault balances:", error);
        process.exit(1);
    }
}

await main();
