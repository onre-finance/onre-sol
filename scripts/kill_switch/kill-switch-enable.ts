import { createKillSwitchTransaction } from "./kill-switch-common";

async function main() {
    try {
        await createKillSwitchTransaction(true); // enable = true
    } catch (error) {
        console.error("Failed to create kill switch enable transaction:", error);
    }
}

main();