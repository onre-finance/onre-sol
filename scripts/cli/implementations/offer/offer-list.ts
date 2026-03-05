import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { printOfferList } from "../../utils/display";

type DecodedOffer = {
    tokenIn: string;
    tokenOut: string;
    offer: any; // Ideally a specific type from the IDL
};

/**
 * Execute offer list command
 */
export async function executeOfferList(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        const { helper } = context;

        // Fetch all accounts matching the offer discriminator.
        // Using raw getProgramAccounts + per-account decode so that stale accounts
        // from previous program versions (different layout/size) are captured and
        // reported rather than crashing the whole command.
        const rawAccounts = await helper.connection.getProgramAccounts(helper.program.programId, {
            filters: [{ memcmp: helper.program.coder.accounts.memcmp("offer") }],
        });

        const valid: DecodedOffer[] = [];
        const legacy: Array<{ address: string; dataSize: number }> = [];

        for (const { pubkey, account } of rawAccounts) {
            try {
                const decoded = helper.program.coder.accounts.decode("offer", account.data);
                valid.push({
                    tokenIn: decoded.tokenInMint.toBase58(),
                    tokenOut: decoded.tokenOutMint.toBase58(),
                    offer: decoded,
                });
            } catch {
                legacy.push({ address: pubkey.toBase58(), dataSize: account.data.length });
            }
        }

        valid.sort((a, b) => a.tokenIn.localeCompare(b.tokenIn) || a.tokenOut.localeCompare(b.tokenOut));

        printOfferList(valid, legacy, opts.json);
    });
}
