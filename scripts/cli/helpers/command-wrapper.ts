import chalk from "chalk";
import { config, ScriptHelper } from "../../utils/script-helper";
import type { GlobalOptions } from "../prompts";
import { ParamDefinition, promptForParams } from "../prompts";
import { printNetworkBanner } from "../utils/display";

/**
 * Context provided to command handlers
 */
export interface CommandContext<T extends Record<string, any>> {
    helper: ScriptHelper;
    params: T;
    opts: GlobalOptions & Record<string, any>;
}

/**
 * Standard command execution wrapper that handles:
 * - Error handling
 * - Network banner printing
 * - ScriptHelper initialization
 * - Parameter prompting
 *
 * This eliminates the boilerplate try-catch + setup code from all command implementations.
 *
 * @param opts - Global options and command-specific options from CLI
 * @param paramDefs - Parameter definitions for prompting
 * @param handler - Async function containing the command logic
 */
export async function executeCommand<T extends Record<string, any>>(
    opts: GlobalOptions & Record<string, any>,
    paramDefs: ParamDefinition[],
    handler: (context: CommandContext<T>) => Promise<void>
): Promise<void> {
    try {
        // Print network banner (unless in JSON mode)
        if (!opts.json) {
            printNetworkBanner(config);
        }

        // Initialize script helper
        const helper = await ScriptHelper.create();

        // Prompt for missing parameters
        const params = await promptForParams(paramDefs, opts, config, opts.noInteractive) as T;

        // Execute the command handler with context
        await handler({ helper, params, opts });

    } catch (error: any) {
        console.error(chalk.red("Error:"), error.message || error);
        process.exit(1);
    }
}
