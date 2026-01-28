import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { ScriptHelper } from "../utils/script-helper";

const TOKEN_PROGRAM = TOKEN_PROGRAM_ID;

async function getCirculatingSupply() {
    const helper = await ScriptHelper.createWithLocalWallet();

    console.log("Fetching circulating supply...");

    try {
        const state = await helper.getState();
        const onycMint = state.onycMint;

        console.log("ONyc Mint:", onycMint.toBase58());

        const vaultOnycAccount = getAssociatedTokenAddressSync(
            onycMint,
            helper.pdas.offerVaultAuthorityPda,
            true,
            TOKEN_PROGRAM
        );

        const circulatingSupply = await helper.program.methods
            .getCirculatingSupply()
            .accounts({
                onycVaultAccount: vaultOnycAccount,
                tokenProgram: TOKEN_PROGRAM
            })
            .view();

        const supplyNumber = circulatingSupply; //.toNumber();

        console.log("\n=== Circulating Supply Results ===");
        console.log(`Circulating Supply (raw): ${supplyNumber}`);
        console.log(`Circulating Supply (formatted): ${supplyNumber.toLocaleString()}`);

        return supplyNumber;
    } catch (error) {
        console.error("Error fetching circulating supply:", error);
        throw error;
    }
}

async function main() {
    try {
        await getCirculatingSupply();
    } catch (error) {
        console.error("Failed to get circulating supply:", error);
    }
}

await main();
