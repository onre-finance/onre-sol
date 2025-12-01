import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { ScriptHelper, USDC_MINT, ONYC_MINT } from "../utils/script-helper";

// Configure which mints to query
const TOKEN_IN_MINT = USDC_MINT;
const TOKEN_OUT_MINT = ONYC_MINT;
const TOKEN_OUT_PROGRAM = TOKEN_2022_PROGRAM_ID; // or TOKEN_PROGRAM_ID

async function getTVL() {
    const helper = await ScriptHelper.createWithLocalWallet();

    console.log("Fetching TVL (Total Value Locked) for offer...");
    console.log("Token In Mint:", TOKEN_IN_MINT.toBase58());
    console.log("Token Out Mint:", TOKEN_OUT_MINT.toBase58());

    try {
        const vaultTokenOutAccount = getAssociatedTokenAddressSync(
            TOKEN_OUT_MINT,
            helper.pdas.offerVaultAuthorityPda,
            true,
            TOKEN_OUT_PROGRAM
        );

        const tvl = await helper.program.methods
            .getTvl()
            .accounts({
                tokenInMint: TOKEN_IN_MINT,
                tokenOutMint: TOKEN_OUT_MINT,
                vaultTokenOutAccount: vaultTokenOutAccount,
                tokenOutProgram: TOKEN_OUT_PROGRAM
            })
            .view();

        const tvlNumber = tvl; //.toNumber();

        console.log("\n=== TVL Results ===");
        console.log(`TVL (raw): ${tvlNumber}`);
        console.log(`TVL (formatted): ${tvlNumber.toLocaleString()}`);

        return tvlNumber;
    } catch (error) {
        console.error("Error fetching TVL:", error);
        throw error;
    }
}

async function main() {
    try {
        await getTVL();
    } catch (error) {
        console.error("Failed to get TVL:", error);
    }
}

await main();
