import { PublicKey } from "@solana/web3.js";
import { ScriptHelper, config } from "../utils/script-helper";

// Token addresses - automatically use the correct mints for the selected network
const TOKEN_IN_MINT = config.mints.usdc;
const TOKEN_OUT_MINT = config.mints.onyc;

async function getAPY() {
    const helper = await ScriptHelper.create();

    console.log("Fetching APY for offer...");
    console.log("Token In Mint:", TOKEN_IN_MINT.toBase58());
    console.log("Token Out Mint:", TOKEN_OUT_MINT.toBase58());

    try {
        const apy = await helper.program.methods
            .getApy()
            .accounts({
                tokenInMint: TOKEN_IN_MINT,
                tokenOutMint: TOKEN_OUT_MINT,
            })
            .view();

        const apyNumber = apy.toNumber();

        console.log("\n=== APY Results ===");
        console.log(`APY (raw): ${apyNumber}`);
        console.log(`APY (%): ${(apyNumber / 1_000_000 * 100).toFixed(4)}%`);

        return apyNumber;
    } catch (error) {
        console.error("Error fetching APY:", error);
        throw error;
    }
}

async function main() {
    try {
        await getAPY();
    } catch (error) {
        console.error("Failed to get APY:", error);
    }
}

await main();
