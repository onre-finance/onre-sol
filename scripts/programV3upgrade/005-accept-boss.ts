import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

const NEW_BOSS = new PublicKey("45YnzauhsBM8CpUz96Djf8UG5vqq2Dua62wuW9H3jaJ5");

async function createAcceptBossTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating accept boss transaction...");
    console.log("New boss (accepting):", NEW_BOSS.toBase58());

    try {
        const state = await helper.getState();
        const currentBoss = state.boss;
        const proposedBoss = state.proposedBoss;

        console.log("Current boss:", currentBoss.toBase58());

        if (proposedBoss) {
            console.log("Proposed boss:", proposedBoss.toBase58());

            if (!proposedBoss.equals(NEW_BOSS)) {
                console.error("\n⚠️  ERROR: The NEW_BOSS in the script does not match the proposed boss in state!");
                console.error(`Script NEW_BOSS: ${NEW_BOSS.toBase58()}`);
                console.error(`State proposed_boss: ${proposedBoss.toBase58()}`);
                throw new Error("NEW_BOSS mismatch");
            }
        } else {
            console.error("\n⚠️  ERROR: No boss proposal found in state!");
            console.error("The current boss must first propose a new boss using propose-boss.ts");
            throw new Error("No proposed boss found");
        }

        const ix = await helper.buildAcceptBossIx({
            newBoss: NEW_BOSS
        });

        const tx = await helper.prepareTransaction({ ix, payer: NEW_BOSS });

        console.log("\nThis is STEP 2 of the two-step ownership transfer:");
        console.log("  1. Current boss proposed a new boss (already completed)");
        console.log("  2. Proposed boss accepts the transfer (this transaction)");
        console.log("\nAfter this transaction, the new boss will become the owner of the program.");

        return helper.printTransaction(tx, "Accept Boss Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createAcceptBossTransaction();
    } catch (error) {
        console.error("Failed to create accept boss transaction:", error);
    }
}

await main();
