import { createKillSwitchTransaction } from "./kill-switch-common";

async function main() {
    try {
        await createKillSwitchTransaction(false); // enable = false
    } catch (error) {
        console.error("Failed to create kill switch disable transaction:", error);
    }
}

main();