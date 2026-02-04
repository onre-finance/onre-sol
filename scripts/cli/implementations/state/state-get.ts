import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { printState } from "../../utils/display";

/**
 * Execute state get command
 */
export async function executeStateGet(opts: GlobalOptions): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        const { helper } = context;

        const state = await helper.program.account.state.fetch(helper.statePda);
        printState(state, opts.json);
    });
}
