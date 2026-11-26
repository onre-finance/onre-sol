import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { printMarketStats } from "../../utils/display";

export async function executeMarketFetch(opts: GlobalOptions): Promise<void> {
    await executeCommand(opts, [], async ({ helper }) => {
        printMarketStats(await helper.getMarketStats(), opts.json);
    });
}
