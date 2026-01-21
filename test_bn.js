import BN from 'bn.js';

// Test 1: Large number without radix
const bn1 = new BN('999999999999999999');
console.log('Test 1 (no radix):', bn1.toString());

// Test 2: Large number with radix 10
const bn2 = new BN('999999999999999999', 10);
console.log('Test 2 (radix 10):', bn2.toString());

// Test 3: From BigInt
const bigInt = BigInt('999999999999999999');
const bn3 = new BN(bigInt.toString());
console.log('Test 3 (BigInt no radix):', bn3.toString());

// Test 4: From BigInt with radix
const bn4 = new BN(bigInt.toString(), 10);
console.log('Test 4 (BigInt + radix):', bn4.toString());

// Test 5: Test with exact failing value
const testValue = '999999999000000000';
const bn5 = new BN(testValue);
console.log('Test 5 (testValue no radix):', bn5.toString());
const bn6 = new BN(testValue, 10);
console.log('Test 6 (testValue radix 10):', bn6.toString());
