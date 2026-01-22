/**
 * BN Wrapper to fix toString() bug on Linux Node 24.10.0
 *
 * This wrapper ensures all BN operations use base 10 for toString()
 */

import OriginalBN from 'bn.js';

export class BN extends OriginalBN {
    toString(base?: number | 'hex', padding?: number): string {
        // Always use base 10 if no base is provided
        if (base === undefined) {
            return super.toString(10, padding);
        }
        return super.toString(base, padding);
    }
}

/**
 * Safely convert BN to BigInt by bypassing the toString() bug.
 * Uses the internal buffer representation instead of toString().
 */
export function bnToBigInt(bn: OriginalBN): bigint {
    // Convert BN to little-endian byte array (8 bytes for u64)
    const buffer = bn.toArrayLike(Buffer, 'le', 8);
    // Read as BigInt using DataView
    const view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);
    return view.getBigUint64(0, true);
}

// Also export as default
export default BN;
