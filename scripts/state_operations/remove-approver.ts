import { PublicKey } from "@solana/web3.js";
import { ScriptHelper } from "../utils/script-helper";

// Approver to remove - UPDATE THIS
const APPROVER = new PublicKey("REPLACE_WITH_APPROVER_PUBKEY");

async function createRemoveApproverTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating remove approver transaction...");
    console.log("Approver to remove:", APPROVER.toBase58());

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        const state = await helper.getState();
        console.log("\nCurrent Approvers:");

        const approver1 = state.approver1;
        const approver2 = state.approver2;

        let approverFound = false;

        if (approver1 && !approver1.equals(PublicKey.default)) {
            console.log("  Approver 1:", approver1.toBase58());
            if (approver1.equals(APPROVER)) {
                approverFound = true;
                console.log("    ↑ This will be removed");
            }
        } else {
            console.log("  Approver 1: Not set");
        }

        if (approver2 && !approver2.equals(PublicKey.default)) {
            console.log("  Approver 2:", approver2.toBase58());
            if (approver2.equals(APPROVER)) {
                approverFound = true;
                console.log("    ↑ This will be removed");
            }
        } else {
            console.log("  Approver 2: Not set");
        }

        if (!approverFound) {
            console.error("\n⚠️  ERROR: The specified approver is not in the state!");
            console.error("Approver:", APPROVER.toBase58());
            throw new Error("Approver not found");
        }

        const ix = await helper.buildRemoveApproverIx({
            approver: APPROVER
        });

        const tx = await helper.prepareTransaction(ix);

        console.log("\nThis will remove the approver from the state.");
        console.log("After removal, this approver will no longer be valid for approval verification.");

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
