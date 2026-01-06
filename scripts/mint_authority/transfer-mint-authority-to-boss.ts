import { config, ScriptHelper } from "../utils/script-helper";
import { getMint, TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Configuration - uses the selected network's ONyc mint
const MINT = config.mints.onyc; // The mint whose authority will be transferred back to boss
const TOKEN_PROGRAM = TOKEN_PROGRAM_ID; // Use TOKEN_2022_PROGRAM_ID for Token-2022 mints

async function createTransferMintAuthorityToBossTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating transfer mint authority to boss transaction...");
    console.log("Mint:", MINT.toBase58());
    console.log("Token Program:", TOKEN_PROGRAM.toBase58());

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());
    console.log("Program Mint Authority PDA:", helper.pdas.mintAuthorityPda.toBase58());

    try {
        // Fetch current mint info
        const mintInfo = await getMint(helper.connection, MINT, "confirmed", TOKEN_PROGRAM);
        console.log("\nCurrent Mint Info:");
        console.log("  Decimals:", mintInfo.decimals);
        console.log("  Supply:", mintInfo.supply.toString());
        console.log("  Current Mint Authority:", mintInfo.mintAuthority?.toBase58() || "None");

        if (!mintInfo.mintAuthority) {
            throw new Error("Mint has no authority - cannot transfer");
        }

        if (mintInfo.mintAuthority.toBase58() !== helper.pdas.mintAuthorityPda.toBase58()) {
            console.warn("\n⚠️  WARNING: Program PDA is not the current mint authority!");
            console.warn(`   Current authority: ${mintInfo.mintAuthority.toBase58()}`);
            console.warn(`   Program PDA: ${helper.pdas.mintAuthorityPda.toBase58()}`);

            if (mintInfo.mintAuthority.toBase58() === boss.toBase58()) {
                console.warn("\n   Boss already has mint authority - no transfer needed");
                return;
            }

            throw new Error("Program PDA must be the current mint authority to transfer it back to boss");
        }

        console.log("\n✓ Program PDA is the current mint authority");
        console.log("\nBuilding transaction to transfer authority back to boss...");

        const ix = await helper.buildTransferMintAuthorityToBossIx({
            mint: MINT,
            tokenProgram: TOKEN_PROGRAM,
            boss
        });

        const tx = await helper.prepareTransaction({ ix, payer: boss });

        console.log("\nThis transaction will:");
        console.log("  1. Transfer mint authority from program PDA back to boss");
        console.log("  2. Give boss direct control over token minting");
        console.log("  3. Emit MintAuthorityTransferredToBossEvent");
        console.log("\nAfter this transaction:");
        console.log(`  - Mint authority: ${boss.toBase58()}`);
        console.log("  - Boss can mint tokens directly");
        console.log("  - Program can no longer mint tokens programmatically");
        console.log("  - Boss can transfer authority back using transfer-mint-authority-to-program script");

        return helper.printTransaction(tx, "Transfer Mint Authority to Boss Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createTransferMintAuthorityToBossTransaction();
    } catch (error) {
        console.error("Failed to create transfer mint authority to boss transaction:", error);
    }
}

await main();
