import { ScriptHelper } from "../utils/script-helper";

async function getStateInfo() {
    const helper = await ScriptHelper.create();

    console.log("Fetching program state...");

    try {
        const state = await helper.getState();

        console.log("\n=== Program State ===");
        console.log("Boss:", state.boss.toBase58());
        console.log("Kill switch enabled:", state.isKilled);
        console.log("\nAdmins:");
        if (state.admins && state.admins.length > 0) {
            state.admins.forEach((admin, index) => {
                console.log(`  ${index + 1}. ${admin.toBase58()}`);
            });
        } else {
            console.log("  No admins found");
        }

        return state;
    } catch (error) {
        console.error("Error fetching state:", error);
        throw error;
    }
}

async function main() {
    try {
        await getStateInfo();
    } catch (error) {
        console.error("Failed to get state:", error);
    }
}

await main();