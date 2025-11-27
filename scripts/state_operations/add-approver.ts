import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// Configuration - UPDATE THIS
// The public key of the account to add as an approver
const APPROVER = new PublicKey("GBR7NtVLiapW8YxebyYf6EYFJJytarj6ixqiXCSq4xth");


async function createAddApproverTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating add approver transaction...");
    console.log("\n=== Approver Configuration ===");
    console.log("New Approver:", APPROVER.toBase58());

    // Validate approver is not default pubkey
    if (APPROVER.equals(PublicKey.default)) {
        console.error("\n❌ ERROR: Cannot add default/zero public key as approver");
        throw new Error("Invalid approver - cannot be default public key");
    }

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        const state = await helper.getState();

        console.log("\n=== Current Approver Slots ===");

        const approver1 = state.approver1;
        const approver2 = state.approver2;

        const slot1Filled = approver1 && !approver1.equals(PublicKey.default);
        const slot2Filled = approver2 && !approver2.equals(PublicKey.default);

        console.log("Slot 1:", slot1Filled ? `✓ ${approver1.toBase58()}` : "⚬ Empty");
        console.log("Slot 2:", slot2Filled ? `✓ ${approver2.toBase58()}` : "⚬ Empty");

        // Check if approver is already added
        if ((slot1Filled && approver1.equals(APPROVER)) || (slot2Filled && approver2.equals(APPROVER))) {
            console.log("\n⚠️  This approver is already configured");
            return;
        }

        // Check if both slots are filled
        if (slot1Filled && slot2Filled) {
            console.error("\n❌ ERROR: Both approver slots are already filled!");
            console.error("You must remove an existing approver before adding a new one.");
            console.error("\nUse the remove-approver script to remove one of:");
            console.error(`  - ${approver1.toBase58()}`);
            console.error(`  - ${approver2.toBase58()}`);
            throw new Error("Both approver slots are filled");
        }

        const targetSlot = !slot1Filled ? "Slot 1" : "Slot 2";
        console.log(`\n✓ Will add approver to: ${targetSlot}`);

        console.log("\nBuilding transaction...");

        const ix = await helper.buildAddApproverIx({
            approver: APPROVER
        });

        const tx = await helper.prepareTransaction(ix);

        console.log("\n=== Transaction Effects ===");
        console.log("This transaction will:");
        console.log(`  1. Add ${APPROVER.toBase58()} to ${targetSlot}`);
        console.log("  2. Enable this approver for approval verification");
        console.log("  3. Emit ApproverAddedEvent");
        console.log("\nAfter this transaction:");
        console.log("  - The approver can sign approval messages for take_offer operations");
        console.log("  - Affects offers where needs_approval = true");

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
