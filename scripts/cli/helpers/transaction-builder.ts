import type { PublicKey, TransactionInstruction } from "@solana/web3.js";
import type { ScriptHelper } from "../../utils/script-helper";
import { handleTransaction } from "../transaction/handler";
import { printParamSummary } from "../utils/display";
import type { CommandContext } from "./command-wrapper";

/**
 * Options for building and handling a transaction
 */
export interface TransactionBuilderOptions {
    /**
     * Function that builds the transaction instruction(s)
     * Can return a single instruction or an array
     */
    buildIx: (helper: ScriptHelper, params: any) => Promise<TransactionInstruction | TransactionInstruction[]>;

    /**
     * Title for the transaction (displayed in console)
     */
    title: string;

    /**
     * Optional description of the transaction
     */
    description?: string;

    /**
     * Fee payer for the transaction
     * Defaults to boss if not provided
     */
    payer?: PublicKey;

    /**
     * If provided, prints a parameter summary before building transaction
     */
    showParamSummary?: {
        title: string;
        params: Record<string, any>;
    };
}

/**
 * Build and handle a transaction with standard flow:
 * 1. Optionally show parameter summary
 * 2. Build instruction(s) using provided builder function
 * 3. Prepare transaction with payer
 * 4. Handle transaction (display, prompt for action, execute)
 *
 * This standardizes the transaction building pattern used across all commands.
 *
 * @param context - Command context with helper, params, and opts
 * @param options - Transaction builder options
 */
export async function buildAndHandleTransaction(
    context: CommandContext<any>,
    options: TransactionBuilderOptions
): Promise<void> {
    const { helper, params, opts } = context;
    const { buildIx, title, description, showParamSummary } = options;

    // Show parameter summary if requested
    if (showParamSummary) {
        printParamSummary(showParamSummary.title, showParamSummary.params);
    }

    // Determine payer (default to boss)
    const payer = options.payer || await helper.getBoss();

    // Build instructions
    const ixOrIxs = await buildIx(helper, params);
    const instructions = Array.isArray(ixOrIxs) ? ixOrIxs : [ixOrIxs];

    // Prepare transaction
    const tx = instructions.length === 1
        ? await helper.prepareTransaction({ ix: instructions[0], payer })
        : await helper.prepareTransactionMultipleIxs({ ixs: instructions, payer });

    // Handle transaction (display, prompt, execute)
    await handleTransaction(tx, helper, {
        title,
        description,
        dryRun: opts.dryRun,
        json: opts.json
    });
}
