// Test to diagnose the BN serialization issue
import BN from 'bn.js';
import { MintLayout } from '@solana/spl-token';

console.log('=== Environment Test ===');
console.log('Node version:', process.version);
console.log('Platform:', process.platform);
console.log('Arch:', process.arch);
console.log('');

console.log('=== BN Tests ===');
// Test 1: Create BN from string
const testValues = [
    '999999999999999999',
    '999999999000000000',
    '1000099998999900000',
    '2000199997999800000'
];

for (const val of testValues) {
    const bn1 = new BN(val);
    const bn2 = new BN(val, 10);
    console.log(`Value: ${val}`);
    console.log(`  BN (no radix): ${bn1.toString()}`);
    console.log(`  BN (radix 10): ${bn2.toString()}`);
    console.log(`  Match: ${bn1.toString() === val && bn2.toString() === val ? 'OK' : 'FAIL'}`);
    console.log('');
}

// Test 2: BigInt conversion
console.log('=== BigInt to BN Tests ===');
const bigInt = BigInt('999999999000000000');
const bn3 = new BN(bigInt.toString());
const bn4 = new BN(bigInt.toString(), 10);
console.log('BigInt value:', bigInt.toString());
console.log('BN (no radix):', bn3.toString());
console.log('BN (radix 10):', bn4.toString());
console.log('Match:', bn3.toString() === bigInt.toString() && bn4.toString() === bigInt.toString() ? 'OK' : 'FAIL');
console.log('');

// Test 3: Buffer to u64
console.log('=== Buffer to u64 Tests ===');
const testU64 = BigInt('999999999000000000');
const buffer = Buffer.alloc(8);
buffer.writeBigUInt64LE(testU64, 0);
console.log('Buffer (hex):', buffer.toString('hex'));
const readBack = buffer.readBigUInt64LE(0);
console.log('Read back:', readBack.toString());
console.log('Match:', readBack === testU64 ? 'OK' : 'FAIL');
console.log('');

// Test 4: Mint Supply simulation
console.log('=== Mint Supply Test ===');
const supply = BigInt('999999999') * BigInt(1000000000); // 999_999_999 * 10^9
console.log('Supply (BigInt):', supply.toString());
const mintBuffer = Buffer.alloc(82); // MINT_SIZE
// Write supply at offset 36 (after mint_authority_option + mint_authority + supply)
mintBuffer.writeUInt32LE(1, 0); // mint_authority_option
mintBuffer.writeBigUInt64LE(supply, 36);
mintBuffer.writeUInt8(9, 44); // decimals
mintBuffer.writeUInt8(1, 45); // is_initialized

try {
    const decoded = MintLayout.decode(mintBuffer);
    console.log('Decoded supply:', decoded.supply.toString());
    console.log('Decoded supply type:', typeof decoded.supply);

    const bn5 = new BN(decoded.supply.toString());
    const bn6 = new BN(decoded.supply.toString(), 10);
    console.log('BN from supply (no radix):', bn5.toString());
    console.log('BN from supply (radix 10):', bn6.toString());
    console.log('Match:', bn5.toString() === supply.toString() ? 'OK' : 'FAIL');
} catch (e) {
    console.log('Error decoding mint:', e.message);
}
