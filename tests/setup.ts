/**
 * Global test setup
 *
 * This file patches bn.js to work correctly on all platforms.
 *
 * Issue: On some platforms (Linux x64 with Node 24.10.0), bn.js has a bug where
 * toString() without an explicit radix corrupts large numbers, producing "NaN"
 * at the end of the string representation.
 *
 * Fix: Patch BN.prototype.toString to always use base 10 when no radix is provided.
 */

import { BN } from '@coral-xyz/anchor';

// Save the original toString method
const originalToString = BN.prototype.toString;

// Patch toString to always use base 10 by default
BN.prototype.toString = function(base?: number | 'hex', padding?: number): string {
    // If no base is provided, use 10
    if (base === undefined) {
        return originalToString.call(this, 10, padding);
    }
    return originalToString.call(this, base, padding);
};

console.log('[test setup] Patched BN.prototype.toString to use base 10 by default');
