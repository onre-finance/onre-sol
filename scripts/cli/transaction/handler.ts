import { Keypair, sendAndConfirmTransaction, Transaction } from "@solana/web3.js";
import { confirm, select } from "@inquirer/prompts";
import chalk from "chalk";
import ora from "ora";
import * as fs from "fs";
import * as os from "os";
import type { ScriptHelper } from "../../utils/script-helper";
import clipboard from "clipboardy";

export interface TransactionResult {
    action: "sent" | "copied" | "cancelled";
    signature?: string;
    base58?: string;
}

export interface TransactionOptions {
    title: string;
    description?: string;
    dryRun?: boolean;
    json?: boolean;
}

/**
 * Handle transaction workflow: display, prompt for action, execute
 */
export async function handleTransaction(
    tx: Transaction,
    helper: ScriptHelper,
    options: TransactionOptions
): Promise<TransactionResult> {
    const { title, description, dryRun, json } = options;

    // Serialize transaction
    const base58 = helper.serializeTransaction(tx);

    // JSON output mode
    if (json) {
        console.log(JSON.stringify({
            title,
            base58,
            feePayer: tx.feePayer?.toBase58(),
            recentBlockhash: tx.recentBlockhash,
            instructionCount: tx.instructions.length
        }, null, 2));
        return { action: "copied", base58 };
    }

    // Display transaction summary
    console.log(chalk.bold.blue(`\n=== ${title} ===`));
    if (description) {
        console.log(chalk.gray(description));
    }
    console.log();

    displayTransactionDetails(tx);

    // Dry run mode - just show transaction
    if (dryRun) {
        console.log(chalk.yellow("\n[Dry Run] Transaction generated but not executed."));
        console.log(chalk.gray("\nBase58:"));
        console.log(base58);
        return { action: "copied", base58 };
    }

    // Interactive selection
    const action = await select({
        message: "What would you like to do?",
        choices: [
            {
                name: "Copy Base58 for Squad multisig",
                value: "copy"
            },
            {
                name: "Sign locally and send to chain",
                value: "sign"
            },
            {
                name: "Cancel",
                value: "cancel"
            }
        ]
    });

    switch (action) {
        case "copy":
            return copyTransaction(base58);

        case "sign":
            return signAndSendTransaction(tx, helper);

        case "cancel":
            console.log(chalk.yellow("\nTransaction cancelled."));
            return { action: "cancelled" };

        default:
            return { action: "cancelled" };
    }
}

/**
 * Display transaction details
 */
function displayTransactionDetails(tx: Transaction): void {
    console.log(chalk.gray("Transaction Details:"));
    console.log(`  Fee Payer:        ${tx.feePayer?.toBase58()}`);
    console.log(`  Recent Blockhash: ${tx.recentBlockhash?.slice(0, 16)}...`);
    console.log(`  Instructions:     ${tx.instructions.length}`);

    tx.instructions.forEach((ix, i) => {
        console.log(chalk.gray(`\n  [${i + 1}] Program: ${ix.programId.toBase58().slice(0, 12)}...`));
        console.log(chalk.gray(`      Accounts: ${ix.keys.length}`));
    });
}

/**
 * Copy transaction to clipboard and display
 */
async function copyTransaction(base58: string): Promise<TransactionResult> {
    // Try to copy to clipboard
    try {
        await clipboard.write(base58);
        console.log(chalk.green("\nTransaction copied to clipboard!"));
    } catch {
        console.log(chalk.yellow("\nCould not copy to clipboard automatically."));
    }

    console.log(chalk.gray("\nBase58 Transaction:"));
    console.log(base58);

    console.log(chalk.blue("\nNext steps:"));
    console.log("  1. Go to your Squad multisig at https://v4.squads.so");
    console.log("  2. Create a new transaction using the transaction builder");
    console.log("  3. Paste this Base58 string");
    console.log("  4. Get required signatures and execute");

    return { action: "copied", base58 };
}

/**
 * Sign transaction locally and send to chain
 */
async function signAndSendTransaction(
    tx: Transaction,
    helper: ScriptHelper
): Promise<TransactionResult> {
    // Prompt for wallet selection
    const walletName = await select({
        message: "Select wallet to sign with:",
        choices: [
            { name: "Default (~/.config/solana/id.json)", value: "id" },
            { name: "Custom wallet file", value: "custom" }
        ]
    });

    let keypairPath: string;
    if (walletName === "custom") {
        const { input } = await import("@inquirer/prompts");
        const customPath = await input({
            message: "Enter path to keypair file:",
            default: "~/.config/solana/id.json"
        });
        keypairPath = customPath.replace("~", os.homedir());
    } else {
        keypairPath = `${os.homedir()}/.config/solana/id.json`;
    }

    // Check if file exists
    if (!fs.existsSync(keypairPath)) {
        console.log(chalk.red(`\nKeypair file not found: ${keypairPath}`));
        return { action: "cancelled" };
    }

    // Load keypair
    let keypair: Keypair;
    try {
        const keypairData = JSON.parse(fs.readFileSync(keypairPath, "utf-8"));
        keypair = Keypair.fromSecretKey(new Uint8Array(keypairData));
    } catch (error) {
        console.log(chalk.red("\nFailed to load keypair:"), error);
        return { action: "cancelled" };
    }

    console.log(chalk.gray(`\nSigning with: ${keypair.publicKey.toBase58()}`));

    // Confirm before sending
    const confirmed = await confirm({
        message: "Send transaction to chain?",
        default: false
    });

    if (!confirmed) {
        console.log(chalk.yellow("\nTransaction cancelled."));
        return { action: "cancelled" };
    }

    const spinner = ora("Sending transaction...").start();

    try {
        // Sign and send
        tx.partialSign(keypair);

        const signature = await sendAndConfirmTransaction(
            helper.connection,
            tx,
            [keypair],
            { commitment: "confirmed" }
        );

        spinner.succeed(chalk.green("Transaction confirmed!"));
        console.log(chalk.gray(`\nSignature: ${signature}`));

        // Determine explorer URL based on network
        const network = helper.networkConfig.name;
        const cluster = network.startsWith("devnet") ? "?cluster=devnet" : "";
        console.log(chalk.blue(`Explorer: https://solscan.io/tx/${signature}${cluster}`));

        return { action: "sent", signature };
    } catch (error: any) {
        spinner.fail(chalk.red("Transaction failed"));
        console.log(chalk.red("\nError:"), error.message || error);

        // Show logs if available
        if (error.logs) {
            console.log(chalk.gray("\nTransaction logs:"));
            error.logs.forEach((log: string) => console.log(chalk.gray(`  ${log}`)));
        }

        return { action: "cancelled" };
    }
}
