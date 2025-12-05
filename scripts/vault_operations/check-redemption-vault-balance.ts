import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { ScriptHelper } from "../utils/script-helper";

// Configuration - UPDATE THESE
const TOKEN_MINT = new PublicKey("5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5"); // ONyc Mint Address
const TOKEN_PROGRAM = TOKEN_PROGRAM_ID; // Use TOKEN_2022_PROGRAM_ID for Token-2022

async function checkRedemptionVaultBalance() {
    const helper = await ScriptHelper.create();

    console.log("=".repeat(80));
    console.log("Checking Redemption Vault Token Balance");
    console.log("=".repeat(80));
    console.log("Token mint:", TOKEN_MINT.toBase58());
    console.log("Token program:", TOKEN_PROGRAM.toBase58());

    try {
        // Get the redemption vault authority PDA
        const redemptionVaultAuthority = PublicKey.findProgramAddressSync(
            [Buffer.from("redemption_offer_vault_authority")],
            helper.program.programId
        )[0];
        console.log("\nRedemption Vault Authority PDA:", redemptionVaultAuthority.toBase58());

        // Get the associated token account address
        const vaultTokenAccount = getAssociatedTokenAddressSync(
            TOKEN_MINT,
            redemptionVaultAuthority,
            true, // allowOwnerOffCurve
            TOKEN_PROGRAM
        );
        console.log("Vault Token Account:", vaultTokenAccount.toBase58());

        // Fetch the token account
        const accountInfo = await helper.connection.getAccountInfo(vaultTokenAccount);

        if (!accountInfo) {
            console.log("\n❌ Vault token account does not exist");
            console.log("The redemption vault has not been initialized for this token.");
            return;
        }

        // Parse token account data
        const tokenAccount = await helper.connection.getTokenAccountBalance(vaultTokenAccount);

        console.log("\n" + "=".repeat(80));
        console.log("BALANCE INFORMATION");
        console.log("=".repeat(80));
        console.log("Raw Balance (lamports):", tokenAccount.value.amount);
        console.log("UI Balance:", tokenAccount.value.uiAmountString);
        console.log("Decimals:", tokenAccount.value.decimals);

        // Additional account details
        const parsedAccountInfo = await helper.connection.getParsedAccountInfo(vaultTokenAccount);
        if (parsedAccountInfo.value && 'parsed' in parsedAccountInfo.value.data) {
            const data = parsedAccountInfo.value.data.parsed.info;
            console.log("\n" + "=".repeat(80));
            console.log("ACCOUNT DETAILS");
            console.log("=".repeat(80));
            console.log("Owner:", data.owner);
            console.log("Mint:", data.mint);
            console.log("State:", data.state);

            if (data.closeAuthority) {
                console.log("Close Authority:", data.closeAuthority);
            }
            if (data.delegate) {
                console.log("Delegate:", data.delegate);
                console.log("Delegated Amount:", data.delegatedAmount?.uiAmountString || "0");
            }
        }

        console.log("\n" + "=".repeat(80));

    } catch (error) {
        console.error("\n❌ Error checking vault balance:", error);
        throw error;
    }
}

async function main() {
    try {
        await checkRedemptionVaultBalance();
    } catch (error) {
        console.error("Failed to check redemption vault balance:", error);
        process.exit(1);
    }
}

await main();
