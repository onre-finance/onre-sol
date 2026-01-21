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

// Also export as default
export default BN;
