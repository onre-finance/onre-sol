import { PublicKey } from "@solana/web3.js";
import { TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";
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
            throw new Error(`Unknown token mint: ${mintAddress}. ` + `Please add explicit decimal configuration for this token.`);
    }
}

/**
 * Get the correct token program ID for a given token mint.
 *
 * Token program mapping:
 * - USDC: Standard Token Program (TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA)
 * - ONyc: Standard Token Program (TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA)
 * - USDG: Token-2022 Program (TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb)
 *
 * @param tokenMint - The token mint public key
 * @returns The token program ID for the mint
 * @throws Error if the token mint is not recognized
 */
export function getTokenProgramId(tokenMint: PublicKey): PublicKey {
    const mintAddress = tokenMint.toBase58();

    switch (mintAddress) {
        case config.mints.usdc.toBase58():
            return TOKEN_PROGRAM_ID;

        case config.mints.usdg.toBase58():
            return TOKEN_2022_PROGRAM_ID;

        case config.mints.onyc.toBase58():
            return TOKEN_PROGRAM_ID;

        default:
            throw new Error(`Unknown token mint: ${mintAddress}. ` + `Please add explicit token program configuration for this token.`);
    }
}
