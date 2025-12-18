import { config, printConfigSummary, ScriptHelper } from "../utils/script-helper";
import { getMint, TOKEN_PROGRAM_ID } from "@solana/spl-token";

async function createTransferMintAuthorityToProgramTransaction() {
    printConfigSummary(config);

    const helper = await ScriptHelper.create();
    const MINT = config.mints.onyc;
    const TOKEN_PROGRAM = TOKEN_PROGRAM_ID;

    console.log("Creating transfer mint authority to program transaction...");
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

        if (mintInfo.mintAuthority.toBase58() !== boss.toBase58()) {
            console.warn("\n⚠️  WARNING: Boss is not the current mint authority!");
            console.warn(`   Current authority: ${mintInfo.mintAuthority.toBase58()}`);
            console.warn(`   Boss: ${boss.toBase58()}`);
            throw new Error("Boss must be the current mint authority to transfer it to the program");
        }

        console.log("\n✓ Boss is the current mint authority");
        console.log("\nBuilding transaction to transfer authority to program PDA...");

        const ix = await helper.buildTransferMintAuthorityToProgramIx({
            mint: MINT,
            tokenProgram: TOKEN_PROGRAM,
            boss
        });

        const tx = await helper.prepareTransaction({ ix, payer: boss });

        console.log("\nThis transaction will:");
        console.log("  1. Transfer mint authority from boss to program PDA");
        console.log("  2. Enable programmatic minting via the program");
        console.log("  3. Emit MintAuthorityTransferredToProgramEvent");
        console.log("\nAfter this transaction:");
        console.log(`  - Mint authority: ${helper.pdas.mintAuthorityPda.toBase58()}`);
        console.log("  - Program can mint tokens programmatically");
        console.log("  - Boss can recover authority using transfer-mint-authority-to-boss script");

        return helper.printTransaction(tx, "Transfer Mint Authority to Program Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createTransferMintAuthorityToProgramTransaction();
    } catch (error) {
        console.error("Failed to create transfer mint authority to program transaction:", error);
    }
}

await main();
