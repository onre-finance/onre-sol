import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { printBufferState } from "../../utils/display";

export async function executeBufferGet(opts: GlobalOptions): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        const { helper } = context;
        const bufferState = await helper.getBufferState();
        printBufferState(bufferState, opts.json);
    });
}
