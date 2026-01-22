/**
 * BN Wrapper to fix toString() bug on Linux Node 24.10.0
 *
 * This wrapper ensures all BN operations use base 10 for toString()
 */

import BN from "bn.js";

/**
 * Safely convert BN to BigInt by bypassing the toString() bug.
 * Uses the internal buffer representation instead of toString().
 */
export function bnToBigInt(bn: BN): bigint {
    // Convert BN to little-endian byte array (8 bytes for u64)
    const buffer = bn.toArrayLike(Buffer, "le", 8);
    // Read as BigInt using DataView
    const view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);
    return view.getBigUint64(0, true);
}
