import { confirm, input } from "@inquirer/prompts";

/**
 * Prompt for confirmation of a dangerous operation
 *
 * @param message - The confirmation message to display
 * @param details - Optional additional details to show
 * @param opts - Optional configuration
 * @param opts.requireExactMatch - If provided, requires user to type this exact string to confirm
 * @returns True if user confirmed, false otherwise
 */
export async function confirmDangerousOperation(message: string, details?: string, opts?: { requireExactMatch?: string }): Promise<boolean> {
    if (details) {
        console.log(details);
    }

    if (opts?.requireExactMatch) {
        const userInput = await input({
            message: `${message} (Type "${opts.requireExactMatch}" to confirm):`,
        });
        return userInput === opts.requireExactMatch;
    }

    return await confirm({
        message,
        default: false,
    });
}
