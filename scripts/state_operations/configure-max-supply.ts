import { ScriptHelper } from "../utils/script-helper";
import { BN } from "bn.js";

const MAX_SUPPLY = new BN(100_000_000).mul(new BN(1e9)); // 100 million ONyc tokens (9 decimals)
const TOKEN_DECIMALS = 9; // ONyc has 9 decimals

async function createConfigureMaxSupplyTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating configure max supply transaction...");

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    try {
        // Fetch current state to show existing max supply
        const state = await helper.getState();

        console.log("\n=== Max Supply Configuration ===");
        console.log("\nCurrent Configuration:");
        console.log("  Raw value:", state.maxSupply.toString());

        // Helper function to format BN values safely (avoids toNumber() overflow)
        const formatSupply = (supply: any): string => {
            if (supply.isZero()) {
                return "Unlimited (no cap)";
            }
            // Divide by 10^decimals using BN arithmetic to avoid overflow
            const divisor = new BN(10).pow(new BN(TOKEN_DECIMALS));
            const tokens = supply.div(divisor);
            // Add thousands separator
            return tokens.toString().replaceAll(/\B(?=(\d{3})+(?!\d))/g, ",") + " tokens";
        };

        const currentFormatted = formatSupply(state.maxSupply);
        console.log("  Formatted:", currentFormatted);

        console.log("\nNew Configuration:");
        console.log("  Raw value:", MAX_SUPPLY.toString());

        const newFormatted = formatSupply(MAX_SUPPLY);
        console.log("  Formatted:", newFormatted);

        // Check if already set to this value
        if (state.maxSupply.eq(MAX_SUPPLY)) {
            console.log("\n⚠️  Max supply is already set to this value - no change needed");
            return;
        }

        console.log("\nBuilding transaction to configure max supply...");

        const ix = await helper.buildConfigureMaxSupplyIx({
            maxSupply: MAX_SUPPLY.toNumber(),
            boss
        });

        const tx = await helper.prepareTransaction({ ix, payer: boss });

        console.log("\n=== Transaction Effects ===");
        console.log("This transaction will:");
        console.log("  1. Update the program state's max_supply field");
        console.log("  2. Apply the new cap to all future minting operations");
        console.log("  3. Emit MaxSupplyConfiguredEvent");

        if (MAX_SUPPLY.isZero()) {
            console.log("\n⚠️  WARNING: Setting max supply to 0 removes the cap");
            console.log("   ONyc tokens will be mintable without limit!");
        } else {
            console.log("\n✓ Future minting will be capped at:", newFormatted);
            console.log("  Any mint operation that would exceed this cap will fail");
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
