import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// Configuration - UPDATE THIS
// The public key of the approver to remove
const APPROVER = new PublicKey("REPLACE_WITH_APPROVER_PUBKEY");

/**
 * Remove an approver from the approval verification list
 *
 * This instruction removes a trusted authority from the approver slots.
 * The approver must exist in either approver1 or approver2 slot, otherwise
 * the instruction will fail.
 *
 * Key points:
 * - Removes approver by setting their slot to default/zero public key
 * - Only the boss can remove approvers
 * - Emits ApproverRemovedEvent for tracking
 *
 * Requirements:
 * - Cannot be the default/zero public key
 * - Approver must currently exist in one of the slots
 *
 * Use cases:
 * - Revoking approval authority
 * - Rotating approver keys
 * - Emergency removal of compromised approvers
 * - Cleaning up unused approver slots
 */
async function createRemoveApproverTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating remove approver transaction...");
    console.log("\n=== Approver Removal ===");
    console.log("Approver to remove:", APPROVER.toBase58());

    // Validate approver is not default pubkey
    if (APPROVER.equals(PublicKey.default)) {
        console.error("\n❌ ERROR: Cannot remove default/zero public key");
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

        let approverSlot: string | null = null;

        if (slot1Filled) {
            const isTarget = approver1.equals(APPROVER);
            console.log(`Slot 1: ${isTarget ? "✗" : "✓"} ${approver1.toBase58()}`);
            if (isTarget) {
                approverSlot = "Slot 1";
                console.log("        ↑ Will be removed");
            }
        } else {
            console.log("Slot 1: ⚬ Empty");
        }

        if (slot2Filled) {
            const isTarget = approver2.equals(APPROVER);
            console.log(`Slot 2: ${isTarget ? "✗" : "✓"} ${approver2.toBase58()}`);
            if (isTarget) {
                approverSlot = "Slot 2";
                console.log("        ↑ Will be removed");
            }
        } else {
            console.log("Slot 2: ⚬ Empty");
        }

        if (!approverSlot) {
            console.error("\n❌ ERROR: The specified approver is not configured!");
            console.error("Approver:", APPROVER.toBase58());
            console.error("\nCurrent configured approvers:");
            if (slot1Filled) console.error(`  - Slot 1: ${approver1.toBase58()}`);
            if (slot2Filled) console.error(`  - Slot 2: ${approver2.toBase58()}`);
            if (!slot1Filled && !slot2Filled) console.error("  - None");
            throw new Error("Approver not found in state");
        }

        console.log(`\n✓ Will remove approver from: ${approverSlot}`);
        console.log("\nBuilding transaction...");

        const ix = await helper.buildRemoveApproverIx({
            approver: APPROVER,
            boss
        });

        const tx = await helper.prepareTransaction({ ix, payer: boss });

        console.log("\n=== Transaction Effects ===");
        console.log("This transaction will:");
        console.log(`  1. Remove ${APPROVER.toBase58()} from ${approverSlot}`);
        console.log(`  2. Set ${approverSlot} to empty (default public key)`);
        console.log("  3. Emit ApproverRemovedEvent");
        console.log("\nAfter this transaction:");
        console.log("  - This approver can no longer sign approval messages");
        console.log("  - Existing approvals from this approver become invalid");
        console.log("  - The slot becomes available for a new approver");

        return helper.printTransaction(tx, "Remove Approver Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createRemoveApproverTransaction();
    } catch (error) {
        console.error("Failed to create remove approver transaction:", error);
    }
}

await main();
