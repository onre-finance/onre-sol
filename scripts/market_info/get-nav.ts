import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// Token mint addresses
const TOKEN_IN_MINT = new PublicKey("YOUR_TOKEN_IN_MINT");
const TOKEN_OUT_MINT = new PublicKey("YOUR_TOKEN_OUT_MINT");

async function getNAV() {
    const helper = await ScriptHelper.create();

    console.log("Fetching NAV (Net Asset Value) for offer...");
    console.log("Token In Mint:", TOKEN_IN_MINT.toBase58());
    console.log("Token Out Mint:", TOKEN_OUT_MINT.toBase58());

    try {
        const nav = await helper.program.methods
            .getNav()
            .accounts({
                tokenInMint: TOKEN_IN_MINT,
                tokenOutMint: TOKEN_OUT_MINT
            })
            .view();

        const navNumber = nav.toNumber();

        console.log("\n=== NAV Results ===");
        console.log(`Current Price (raw): ${navNumber}`);
        console.log(`Current Price (decimal): ${(navNumber / 1_000_000_000).toFixed(9)}`);

        return navNumber;
    } catch (error) {
        console.error("Error fetching NAV:", error);
        throw error;
    }
}

async function main() {
    try {
        await getNAV();
    } catch (error) {
        console.error("Failed to get NAV:", error);
    }
}

await main();
