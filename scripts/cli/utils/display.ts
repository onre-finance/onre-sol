import chalk from "chalk";
import Table from "cli-table3";
import type { NetworkConfig } from "../../utils/script-helper";

/**
 * Print network banner
 */
export function printNetworkBanner(config: NetworkConfig): void {
    console.log(chalk.bold.blue(`\n=== Network: ${config.name.toUpperCase()} ===`));
    console.log(chalk.whiteBright(`RPC:     ${config.rpcUrl}`));
    console.log(chalk.whiteBright(`Program: ${config.programId.toBase58()}`));
    console.log(chalk.whiteBright(`Boss:    ${config.boss.toBase58()}`));
    console.log();
}

/**
 * Print program state
 */
export function printState(state: any, json: boolean = false): void {
    if (json) {
        console.log(
            JSON.stringify(
                {
                    boss: state.boss.toBase58(),
                    proposedBoss: state.proposedBoss.toBase58(),
                    isKilled: state.isKilled,
                    onycMint: state.onycMint.toBase58(),
                    maxSupply: state.maxSupply?.toString() || "0",
                    approver1: state.approver1.toBase58(),
                    approver2: state.approver2.toBase58(),
                    redemptionAdmin: state.redemptionAdmin?.toBase58(),
                    admins: state.admins?.map((a: any) => a.toBase58()) || [],
                },
                null,
                2,
            ),
        );
        return;
    }

    console.log(chalk.bold.blue("\n=== Program State ===\n"));

    const table = new Table({
        head: [chalk.white("Field"), chalk.white("Value")],
        colWidths: [20, 50],
    });

    table.push(
        ["Boss", state.boss.toBase58()],
        ["Proposed Boss", state.proposedBoss.toBase58()],
        ["Kill Switch", state.isKilled ? chalk.red("ENABLED") : chalk.green("Disabled")],
        ["ONyc Mint", state.onycMint.toBase58()],
        ["Max Supply", state.maxSupply?.toString() || "Not set"],
        ["Approver 1", state.approver1.toBase58()],
        ["Approver 2", state.approver2.toBase58()],
    );

    if (state.redemptionAdmin) {
        table.push(["Redemption Admin", state.redemptionAdmin.toBase58()]);
    }

    console.log(table.toString());

    // Print admins if any
    if (state.admins && state.admins.length > 0) {
        console.log(chalk.bold("\nAdmins:"));
        state.admins.forEach((admin: any, i: number) => {
            const pubkey = admin.toBase58();
            if (pubkey !== "11111111111111111111111111111111") {
                console.log(`  ${i + 1}. ${pubkey}`);
            }
        });
    }
}

/**
 * Print CACHE state
 */
export function printCacheState(cacheState: any, json: boolean = false): void {
    if (json) {
        console.log(
            JSON.stringify(
                {
                    onycMint: cacheState.onycMint.toBase58(),
                    cacheAdmin: cacheState.cacheAdmin.toBase58(),
                    grossYield: cacheState.grossYield.toString(),
                    currentYield: cacheState.currentYield.toString(),
                    lowestSupply: cacheState.lowestSupply.toString(),
                    lastAccrualTimestamp: cacheState.lastAccrualTimestamp.toString(),
                },
                null,
                2,
            ),
        );
        return;
    }

    console.log(chalk.bold.blue("\n=== CACHE State ===\n"));

    const table = new Table({
        head: [chalk.white("Field"), chalk.white("Value")],
        colWidths: [24, 52],
    });

    table.push(
        ["ONyc Mint", cacheState.onycMint.toBase58()],
        ["CACHE Admin", cacheState.cacheAdmin.toBase58()],
        ["Gross Yield (1e6)", cacheState.grossYield.toString()],
        ["Current Yield (1e6)", cacheState.currentYield.toString()],
        ["Lowest Supply", cacheState.lowestSupply.toString()],
        ["Last Accrual Timestamp", cacheState.lastAccrualTimestamp.toString()],
    );

    console.log(table.toString());
}

/**
 * Print offer details
 */
export function printOffer(offer: any, tokenInMint: string, tokenOutMint: string, json: boolean = false): void {
    if (json) {
        console.log(
            JSON.stringify(
                {
                    tokenInMint,
                    tokenOutMint,
                    feeBasisPoints: offer.feeBasisPoints,
                    needsApproval: offer.needsApproval,
                    allowPermissionless: offer.allowPermissionless,
                    vectors:
                        offer.vectors?.map((v: any) => ({
                            baseTime: v.baseTime.toString(),
                            basePrice: v.basePrice.toString(),
                            apr: v.apr.toString(),
                            priceFixDuration: v.priceFixDuration.toString(),
                        })) || [],
                },
                null,
                2,
            ),
        );
        return;
    }

    console.log(chalk.bold.blue("\n=== Offer Details ===\n"));

    const table = new Table({
        head: [chalk.white("Field"), chalk.white("Value")],
        colWidths: [25, 50],
    });

    table.push(
        ["Token In Mint", tokenInMint],
        ["Token Out Mint", tokenOutMint],
        ["Fee", `${offer.feeBasisPoints / 100}% (${offer.feeBasisPoints} bps)`],
        ["Needs Approval", offer.needsApproval ? "Yes" : "No"],
        ["Allow Permissionless", offer.allowPermissionless ? "Yes" : "No"],
    );

    console.log(table.toString());

    // Print vectors
    if (offer.vectors && offer.vectors.length > 0) {
        console.log(chalk.bold("\nPricing Vectors:"));

        const vectorTable = new Table({
            head: [chalk.white("#"), chalk.white("Base Time"), chalk.white("Base Price"), chalk.white("APR"), chalk.white("Duration")],
            colWidths: [4, 24, 16, 12, 12],
        });

        offer.vectors.forEach((v: any, i: number) => {
            const baseTime = new Date(v.baseTime.toNumber() * 1000).toISOString();
            const basePrice = (v.basePrice.toNumber() / 1_000_000_000).toFixed(9);
            const apr = (v.apr.toNumber() / 10000).toFixed(2) + "%";
            const duration = formatDuration(v.priceFixDuration.toNumber());

            vectorTable.push([(i + 1).toString(), baseTime.replace("T", " ").replace(".000Z", ""), basePrice, apr, duration]);
        });

        console.log(vectorTable.toString());
    } else {
        console.log(chalk.yellow("\nNo pricing vectors configured."));
    }
}

/**
 * Print NAV result
 */
export function printNav(nav: number, json: boolean = false): void {
    const navDecimal = (nav / 1_000_000_000).toFixed(9);

    if (json) {
        console.log(JSON.stringify({ nav, navDecimal }, null, 2));
        return;
    }

    console.log(chalk.bold.blue("\n=== NAV (Net Asset Value) ===\n"));
    console.log(`  Raw Value:     ${nav}`);
    console.log(`  Decimal:       ${navDecimal}`);
    console.log(`  Display:       $${parseFloat(navDecimal).toFixed(4)}`);
}

/**
 * Print NAV adjustmentresult
 */
export function printNavAdjustment(nav: number, json: boolean = false): void {
    const navDecimal = (nav / 1_000_000_000).toFixed(9);

    if (json) {
        console.log(JSON.stringify({ nav, navDecimal }, null, 2));
        return;
    }

    console.log(chalk.bold.blue("\n=== NAV (Net Asset Value) Adjustment ===\n"));
    console.log(`  Raw Value:     ${nav}`);
    console.log(`  Decimal:       ${navDecimal}`);
    console.log(`  Display:       $${parseFloat(navDecimal).toFixed(4)}`);
}

/**
 * Print APY result
 */
export function printApy(apy: number, json: boolean = false): void {
    const apyPercent = (apy / 10000).toFixed(4);

    if (json) {
        console.log(JSON.stringify({ apy, apyPercent: parseFloat(apyPercent) }, null, 2));
        return;
    }

    console.log(chalk.bold.blue("\n=== APY ===\n"));
    console.log(`  Raw Value:     ${apy}`);
    console.log(`  Percentage:    ${apyPercent}%`);
}

/**
 * Print TVL result
 */
export function printTvl(tvl: number | string, json: boolean = false): void {
    const tvlNum = typeof tvl === "string" ? parseFloat(tvl) : tvl;

    if (json) {
        console.log(JSON.stringify({ tvl: tvl.toString(), tvlUsdc: tvlNum / 1_000_000 }, null, 2));
        return;
    }

    console.log(chalk.bold.blue("\n=== TVL (Total Value Locked) ===\n"));
    console.log(`  Raw Value:     ${tvl}`);
    console.log(`  USDC:          ${(tvlNum / 1_000_000).toLocaleString()} USDC`);
}

/**
 * Print circulating supply
 */
export function printCirculatingSupply(supply: number | string, json: boolean = false): void {
    const supplyNum = typeof supply === "string" ? parseFloat(supply) : supply;

    if (json) {
        console.log(JSON.stringify({ supply: supply.toString(), supplyTokens: supplyNum / 1_000_000_000 }, null, 2));
        return;
    }

    console.log(chalk.bold.blue("\n=== Circulating Supply ===\n"));
    console.log(`  Raw Value:     ${supply}`);
    console.log(`  Tokens:        ${(supplyNum / 1_000_000_000).toLocaleString()}`);
}

/**
 * Format duration in seconds to human readable
 */
function formatDuration(seconds: number): string {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
    return `${Math.floor(seconds / 86400)}d`;
}

/**
 * Print a summary of parameters before executing a command
 */
export function printParamSummary(title: string, params: Record<string, any>): void {
    console.log(chalk.bold.blue(`\n${title}\n`));

    for (const [key, value] of Object.entries(params)) {
        const displayValue = formatParamValue(value);
        console.log(`  ${chalk.gray(formatParamName(key) + ":")} ${displayValue}`);
    }
    console.log();
}

/**
 * Format parameter name from camelCase to Title Case
 */
function formatParamName(name: string): string {
    return name
        .replace(/([A-Z])/g, " $1")
        .replace(/^./, (str) => str.toUpperCase())
        .trim();
}

/**
 * Format parameter value for display
 */
function formatParamValue(value: any): string {
    if (value === null || value === undefined) {
        return chalk.gray("(not set)");
    }
    if (typeof value === "boolean") {
        return value ? chalk.green("Yes") : chalk.gray("No");
    }
    if (value.toBase58) {
        return value.toBase58();
    }
    if (typeof value === "number") {
        return value.toLocaleString();
    }
    return String(value);
}

/**
 * Print redemption offer details
 */
export function printRedemptionOffer(offer: any, tokenInMint: string, tokenOutMint: string, json: boolean = false): void {
    if (json) {
        console.log(
            JSON.stringify(
                {
                    tokenInMint,
                    tokenOutMint,
                    feeBasisPoints: offer.feeBasisPoints,
                    requestCounter: offer.requestCounter.toString(),
                    executedRedemptions: offer.executedRedemptions.toString(),
                    requestedRedemptions: offer.requestedRedemptions.toString(),
                    offer: offer.offer.toBase58(),
                },
                null,
                2,
            ),
        );
        return;
    }

    console.log(chalk.bold.blue("\n=== Redemption Offer Details ===\n"));

    const table = new Table({
        head: [chalk.white("Field"), chalk.white("Value")],
        colWidths: [25, 50],
    });

    table.push(
        ["Token In Mint", tokenInMint],
        ["Token Out Mint", tokenOutMint],
        ["Fee", `${offer.feeBasisPoints / 100}% (${offer.feeBasisPoints} bps)`],
        ["Underlying Offer", offer.offer.toBase58()],
        ["Request Counter", offer.requestCounter.toString()],
        ["Executed Redemptions", offer.executedRedemptions.toString()],
        ["Requested Redemptions", offer.requestedRedemptions.toString()],
    );

    console.log(table.toString());
}

/**
 * Print redemption request details
 */
export function printRedemptionRequest(request: any, requestId: number, json: boolean = false): void {
    const fulfilledAmount = request.fulfilledAmount ?? 0;
    const remaining = BigInt(request.amount.toString()) - BigInt(fulfilledAmount.toString());

    if (json) {
        console.log(
            JSON.stringify(
                {
                    requestId,
                    offer: request.offer.toBase58(),
                    redeemer: request.redeemer.toBase58(),
                    amount: request.amount.toString(),
                    fulfilledAmount: fulfilledAmount.toString(),
                    remainingAmount: remaining.toString(),
                },
                null,
                2,
            ),
        );
        return;
    }

    console.log(chalk.bold.blue("\n=== Redemption Request Details ===\n"));

    const table = new Table({
        head: [chalk.white("Field"), chalk.white("Value")],
        colWidths: [20, 50],
    });

    table.push(
        ["Request ID", requestId.toString()],
        ["Redemption Offer", request.offer.toBase58()],
        ["Redeemer", request.redeemer.toBase58()],
        ["Total Amount", request.amount.toString()],
        ["Fulfilled Amount", fulfilledAmount.toString()],
        ["Remaining Amount", remaining.toString()],
    );

    console.log(table.toString());
}

/**
 * Print redemption requests list
 */
export function printRedemptionRequestsList(requests: Array<{ id: number; request: any }>, json: boolean = false): void {
    if (json) {
        console.log(
            JSON.stringify(
                requests.map((r) => {
                    const fulfilledAmount = r.request.fulfilledAmount ?? 0;
                    const remaining = BigInt(r.request.amount.toString()) - BigInt(fulfilledAmount.toString());
                    return {
                        requestId: r.id,
                        offer: r.request.offer.toBase58(),
                        redeemer: r.request.redeemer.toBase58(),
                        amount: r.request.amount.toString(),
                        fulfilledAmount: fulfilledAmount.toString(),
                        remainingAmount: remaining.toString(),
                    };
                }),
                null,
                2,
            ),
        );
        return;
    }

    if (requests.length === 0) {
        console.log(chalk.yellow("\nNo pending redemption requests found."));
        return;
    }

    console.log(chalk.bold.blue(`\n=== Redemption Requests (${requests.length} found) ===\n`));

    const table = new Table({
        head: [chalk.white("ID"), chalk.white("Redeemer"), chalk.white("Total"), chalk.white("Fulfilled"), chalk.white("Remaining")],
        colWidths: [6, 46, 18, 18, 18],
    });

    requests.forEach(({ id, request }) => {
        const fulfilledAmount = request.fulfilledAmount ?? 0;
        const remaining = BigInt(request.amount.toString()) - BigInt(fulfilledAmount.toString());
        table.push([
            id.toString(),
            request.redeemer.toBase58(),
            request.amount.toString(),
            fulfilledAmount.toString(),
            remaining.toString(),
        ]);
    });

    console.log(table.toString());
}
