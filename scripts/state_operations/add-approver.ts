import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// Approver to add - UPDATE THIS
const APPROVER = new PublicKey("GBR7NtVLiapW8YxebyYf6EYFJJytarj6ixqiXCSq4xth");

async function createAddApproverTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating add approver transaction...");
    console.log("Approver:", APPROVER.toBase58());

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        const state = await helper.getState();
        console.log("\nCurrent Approvers:");

        const approver1 = state.approver1;
        const approver2 = state.approver2;

        if (approver1 && !approver1.equals(PublicKey.default)) {
            console.log("  Approver 1:", approver1.toBase58());
        } else {
            console.log("  Approver 1: Not set");
        }

        if (approver2 && !approver2.equals(PublicKey.default)) {
            console.log("  Approver 2:", approver2.toBase58());
        } else {
            console.log("  Approver 2: Not set");
        }

        // Check if both slots are filled
        const slot1Filled = approver1 && !approver1.equals(PublicKey.default);
        const slot2Filled = approver2 && !approver2.equals(PublicKey.default);

        if (slot1Filled && slot2Filled) {
            console.error("\n⚠️  ERROR: Both approver slots are already filled!");
            console.error("You must remove an existing approver before adding a new one.");
            throw new Error("Both approver slots are filled");
        }

        const ix = await helper.buildAddApproverIx({
            approver: APPROVER
        });

        const tx = await helper.prepareTransaction(ix);

        console.log("\nThis will add the approver to an available slot.");
        console.log("The approver can be used for approval verification in take_offer operations.");

        return helper.printTransaction(tx, "Add Approver Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createAddApproverTransaction();
    } catch (error) {
        console.error("Failed to create add approver transaction:", error);
    }
}

await main();
