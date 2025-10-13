import { getAssociatedTokenAddressSync, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { ScriptHelper, TOKEN_IN_MINT, TOKEN_OUT_MINT } from "../utils/script-helper";

const TOKEN_OUT_PROGRAM = TOKEN_2022_PROGRAM_ID; // or TOKEN_PROGRAM_ID

async function getTVL() {
    const helper = await ScriptHelper.create();

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

        const tvlNumber = tvl.toNumber();

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
