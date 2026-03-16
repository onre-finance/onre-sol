import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
import { bufferBurnParams } from "../../params";
import { EventParser } from "@coral-xyz/anchor";

export async function executeBufferBurn(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, bufferBurnParams, async (context) => {
        const { helper, params } = context;

        const boss = await helper.getBoss();

        if (opts.simulate) {
            const ix = await helper.buildBurnForNavIncreaseIx({
                boss,
                tokenInMint: params.tokenIn,
                onycMint: params.onycMint,
                assetAdjustmentAmount: params.assetAdjustmentAmount,
                targetNav: params.targetNav,
            });

            const tx = await helper.prepareTransaction({ ix, payer: boss });
            const simulation = await helper.connection.simulateTransaction(tx);

            if (simulation.value.err) {
                throw new Error(
                    `Simulation failed: ${JSON.stringify(simulation.value.err)}${
                        simulation.value.logs?.length ? `\n${simulation.value.logs.join("\n")}` : ""
                    }`,
                );
            }

            const parser = new EventParser(helper.program.programId, helper.program.coder);
            let burnEvent: any | undefined;
            const events = parser.parseLogs(simulation.value.logs ?? []);
            let currentEvent = events.next();
            while (!currentEvent.done) {
                const event = currentEvent.value;
                if (event.name === "bufferBurnedForNavEvent") {
                    burnEvent = event.data;
                    break;
                }
                currentEvent = events.next();
            }

            if (!burnEvent) {
                throw new Error("Simulation succeeded but BufferBurnedForNavEvent was not found in logs");
            }

            if (opts.json) {
                console.log(
                    JSON.stringify(
                        {
                            mode: "simulate",
                            burnAmount: burnEvent.burnAmount.toString(),
                            assetAdjustmentAmount: burnEvent.assetAdjustmentAmount.toString(),
                            totalAssets: burnEvent.totalAssets.toString(),
                            targetNav: burnEvent.targetNav.toString(),
                            unitsConsumed: simulation.value.unitsConsumed ?? null,
                        },
                        null,
                        2,
                    ),
                );
                return;
            }

            console.log("\n=== Burn For NAV Increase Simulation ===\n");
            console.log(`  Burn Amount (raw):        ${burnEvent.burnAmount.toString()}`);
            console.log(`  Asset Adjustment (raw):   ${burnEvent.assetAdjustmentAmount.toString()}`);
            console.log(`  Total Assets (raw):       ${burnEvent.totalAssets.toString()}`);
            console.log(`  Target NAV (raw):         ${burnEvent.targetNav.toString()}`);
            if (simulation.value.unitsConsumed !== undefined) {
                console.log(`  Compute Units Consumed:   ${simulation.value.unitsConsumed}`);
            }
            console.log("\nSimulation only. No transaction was sent.");
            return;
        }

        await buildAndHandleTransaction(context, {
            buildIx: async (helper) => {
                return helper.buildBurnForNavIncreaseIx({
                    boss,
                    tokenInMint: params.tokenIn,
                    onycMint: params.onycMint,
                    assetAdjustmentAmount: params.assetAdjustmentAmount,
                    targetNav: params.targetNav,
                });
            },
            title: "Burn For NAV Increase Transaction",
            description: "Burns ONyc from BUFFER vault to support NAV adjustment",
            showParamSummary: {
                title: "Burning from BUFFER:",
                params: {
                    tokenInMint: params.tokenIn,
                    onycMint: params.onycMint,
                    assetAdjustmentAmount: params.assetAdjustmentAmount,
                    targetNav: params.targetNav,
                },
            },
        });
    });
}
