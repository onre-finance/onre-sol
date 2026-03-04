import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { printRedemptionOfferList } from "../../utils/display";

/**
 * Execute redemption list-offers command
 */
export async function executeRedemptionListOffers(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, [], async (context) => {
        const { helper } = context;

        // Fetch all accounts matching the redemptionOffer discriminator.
        // Per-account decode with try/catch so stale accounts from previous
        // program versions are captured and reported rather than crashing.
        const rawAccounts = await helper.connection.getProgramAccounts(helper.program.programId, {
            filters: [{ memcmp: helper.program.coder.accounts.memcmp("redemptionOffer") }],
        });

        const valid: Array<{ tokenIn: string; tokenOut: string; offer: any }> = [];
        const legacy: Array<{ address: string; dataSize: number }> = [];

        for (const { pubkey, account } of rawAccounts) {
            try {
                const decoded = helper.program.coder.accounts.decode("redemptionOffer", account.data);
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

        printRedemptionOfferList(valid, legacy, opts.json);
    });
}
