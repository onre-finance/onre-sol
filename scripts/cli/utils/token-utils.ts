import { PublicKey } from "@solana/web3.js";
import { config } from "../../utils/script-helper";

/**
 * Get the number of decimals for a given token mint.
 *
 * Each token must have explicitly defined decimals:
 * - USDC: 6 decimals
 * - USDG: 6 decimals
 * - ONyc: 9 decimals
 *
 * @param tokenMint - The token mint public key
 * @returns The number of decimals for the token
 * @throws Error if the token mint is not recognized
 */
export function getTokenDecimals(tokenMint: PublicKey): number {
    const mintAddress = tokenMint.toBase58();

    switch (mintAddress) {
        case config.mints.usdc.toBase58():
            return 6;

        case config.mints.usdg.toBase58():
            return 6;

        case config.mints.onyc.toBase58():
            return 9;

        default:
            throw new Error(
                `Unknown token mint: ${mintAddress}. ` +
                `Please add explicit decimal configuration for this token.`
            );
    }
}
