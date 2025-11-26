import { ScriptHelper } from "../utils/script-helper";
import { BN } from "bn.js";

// Maximum supply configuration - UPDATE THIS
// Set to 0 to remove the cap (unlimited supply)
// Otherwise, specify the maximum supply in base units (considering token decimals)
// Example: For a token with 9 decimals, 100_000_000 * 1e9 = 100 million tokens
const MAX_SUPPLY = new BN(100_000_000).mul(new BN(1e9)); // 100 million tokens with 9 decimals
// const MAX_SUPPLY = new BN(0); // 0 = no cap

async function createConfigureMaxSupplyTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating configure max supply transaction...");
    console.log("Max Supply:", MAX_SUPPLY.isZero() ? "No cap (unlimited)" : MAX_SUPPLY.toString());

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        const state = await helper.getState();
        console.log("\nCurrent State:");
        console.log("  Current Max Supply:", state.maxSupply?.toString() || "Not set");

        const ix = await helper.buildConfigureMaxSupplyIx({
            maxSupply: MAX_SUPPLY
        });

        const tx = await helper.prepareTransaction(ix);

        if (MAX_SUPPLY.isZero()) {
            console.log("\n⚠️  This will REMOVE the maximum supply cap!");
            console.log("ONyc tokens will be mintable without limit.");
        } else {
            console.log("\n⚠️  This will set the maximum supply cap to:", MAX_SUPPLY.toString());
            console.log("ONyc token minting will be restricted by this cap.");
        }

        return helper.printTransaction(tx, "Configure Max Supply Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createConfigureMaxSupplyTransaction();
    } catch (error) {
        console.error("Failed to create configure max supply transaction:", error);
    }
}

await main();
