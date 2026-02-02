import { PublicKey } from "@solana/web3.js";
import { config } from "../../utils/script-helper";

/**
 * Get the number of decimals for a given token mint.
 *
 * USDC and USDG use 6 decimals.
 * ONyc and all other tokens use 9 decimals.
 *
 * @param tokenMint - The token mint public key
 * @returns The number of decimals for the token
 */
export function getTokenDecimals(tokenMint: PublicKey): number {
    // USDC and USDG have 6 decimals
    if (tokenMint.equals(config.mints.usdc) || tokenMint.equals(config.mints.usdg)) {
        return 6;
    }

    // ONyc and all other tokens have 9 decimals
    return 9;
}
