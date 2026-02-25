import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { printCacheState } from "../../utils/display";

export async function executeCacheGet(opts: GlobalOptions): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        const { helper } = context;
        const cacheState = await helper.getCacheState();
        printCacheState(cacheState, opts.json);
    });
}
