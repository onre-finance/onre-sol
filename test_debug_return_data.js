// Test to debug return data handling
import { TestHelper } from "./tests/test_helper.ts";
import { OnreProgram } from "./tests/onre_program.ts";
import BN from 'bn.js';

console.log('=== Return Data Debug Test ===\n');

const testHelper = await TestHelper.create();
const program = new OnreProgram(testHelper);

// Create mints
const tokenOutMint = testHelper.createMint(9);

// Initialize program
await program.initialize({ onycMint: tokenOutMint });

console.log('Getting circulating supply...\n');

// Get circulating supply - let's intercept the flow
const tokenOutProgram = testHelper.svm.getAccount(tokenOutMint).owner;
const { TOKEN_PROGRAM_ID } = await import('@solana/spl-token');
const { getAssociatedTokenAddressSync } = await import('@solana/spl-token');

const tx = await program.program.methods
    .getCirculatingSupply()
    .accounts({
        tokenProgram: TOKEN_PROGRAM_ID,
        onycVaultAccount: getAssociatedTokenAddressSync(tokenOutMint, program.pdas.offerVaultAuthorityPda, true, TOKEN_PROGRAM_ID)
    })
    .transaction();

tx.recentBlockhash = testHelper.svm.latestBlockhash();
tx.feePayer = testHelper.payer.publicKey;
tx.sign(testHelper.payer);

const result = testHelper.svm.simulateTransaction(tx);

if ("Err" in result || typeof result.err === 'function') {
    console.log('Error in simulation');
    process.exit(1);
}

const meta = result.meta();
const returnData = meta.returnData();

if (!returnData || returnData.data().length === 0) {
    console.log('No return data');
    process.exit(1);
}

const data = returnData.data();
console.log('Return data type:', data.constructor.name);
console.log('Return data length:', data.length);
console.log('Return data (hex):', Buffer.from(data).toString('hex'));
console.log('Return data (bytes):', Array.from(data));
console.log('');

// Test different ways to read the data
console.log('=== Reading as DataView ===');
const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
console.log('DataView buffer:', view.buffer);
console.log('DataView byteOffset:', view.byteOffset);
console.log('DataView byteLength:', view.byteLength);

const asUint64 = view.getBigUint64(0, true);
console.log('As BigUint64LE:', asUint64);
console.log('BigUint64 type:', typeof asUint64);
console.log('BigUint64 toString():', asUint64.toString());
console.log('');

// Test BN creation
console.log('=== Creating BN ===');
const bn1 = new BN(asUint64.toString());
console.log('BN (no radix):', bn1);
console.log('BN toString():', bn1.toString());
console.log('BN toString() type:', typeof bn1.toString());
console.log('BN toString() length:', bn1.toString().length);
console.log('');

const bn2 = new BN(asUint64.toString(), 10);
console.log('BN (radix 10):', bn2);
console.log('BN toString():', bn2.toString());
console.log('BN toString() type:', typeof bn2.toString());
console.log('BN toString() length:', bn2.toString().length);
console.log('');

// Try reading directly from buffer
console.log('=== Reading from Buffer ===');
const buffer = Buffer.from(data);
const fromBuffer = buffer.readBigUInt64LE(0);
console.log('From Buffer:', fromBuffer);
console.log('From Buffer toString():', fromBuffer.toString());
const bn3 = new BN(fromBuffer.toString(), 10);
console.log('BN from buffer:', bn3.toString());
console.log('');

// Compare with mint info
console.log('=== Comparing with Mint Info ===');
const mintInfo = await testHelper.getMintInfo(tokenOutMint);
console.log('Mint supply:', mintInfo.supply);
console.log('Mint supply type:', typeof mintInfo.supply);
console.log('Mint supply toString():', mintInfo.supply.toString());
console.log('Match:', asUint64 === mintInfo.supply ? 'YES' : 'NO');
