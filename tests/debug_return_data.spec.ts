import { TestHelper } from "./test_helper";
import { OnreProgram } from "./onre_program.ts";
import { BN } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from "@solana/spl-token";

describe("Debug Return Data", () => {
    it("Should debug circulating supply return data", async () => {
        console.log('\n=== Return Data Debug Test ===\n');

        const testHelper = await TestHelper.create();
        const program = new OnreProgram(testHelper);

        // Create mints
        const tokenOutMint = testHelper.createMint(9);

        // Initialize program
        await program.initialize({ onycMint: tokenOutMint });

        console.log('Getting circulating supply...\n');

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
            throw new Error('Simulation failed');
        }

        const meta = result.meta();
        const returnData = meta.returnData();

        if (!returnData || returnData.data().length === 0) {
            console.log('No return data');
            throw new Error('No return data');
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
        console.log('DataView byteOffset:', view.byteOffset);
        console.log('DataView byteLength:', view.byteLength);
        console.log('DataView buffer byteLength:', view.buffer.byteLength);

        const asUint64 = view.getBigUint64(0, true);
        console.log('As BigUint64LE:', asUint64);
        console.log('BigUint64 type:', typeof asUint64);
        console.log('BigUint64 toString():', asUint64.toString());
        console.log('BigUint64 toString() length:', asUint64.toString().length);
        console.log('');

        // Test BN creation
        console.log('=== Creating BN ===');
        const bn1 = new BN(asUint64.toString());
        console.log('BN (no radix) toString():', bn1.toString());
        console.log('BN toString() type:', typeof bn1.toString());
        console.log('BN toString() length:', bn1.toString().length);
        console.log('BN words:', bn1.words);
        console.log('BN negative:', bn1.negative);
        console.log('');

        const bn2 = new BN(asUint64.toString(), 10);
        console.log('BN (radix 10) toString():', bn2.toString());
        console.log('BN toString() type:', typeof bn2.toString());
        console.log('BN toString() length:', bn2.toString().length);
        console.log('BN words:', bn2.words);
        console.log('BN negative:', bn2.negative);
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

        // Try creating DataView from fresh buffer
        console.log('=== Fresh Buffer Copy ===');
        const freshBuffer = Buffer.allocUnsafe(8);
        freshBuffer.writeBigUInt64LE(fromBuffer, 0);
        const freshView = new DataView(freshBuffer.buffer, freshBuffer.byteOffset, freshBuffer.byteLength);
        const freshUint64 = freshView.getBigUint64(0, true);
        console.log('Fresh BigUint64:', freshUint64);
        const bn4 = new BN(freshUint64.toString(), 10);
        console.log('BN from fresh:', bn4.toString());
        console.log('');

        // Compare with mint info
        console.log('=== Comparing with Mint Info ===');
        const mintInfo = await testHelper.getMintInfo(tokenOutMint);
        console.log('Mint supply:', mintInfo.supply);
        console.log('Mint supply type:', typeof mintInfo.supply);
        console.log('Mint supply toString():', mintInfo.supply.toString());
        console.log('Match:', asUint64 === mintInfo.supply ? 'YES' : 'NO');
        console.log('');

        // Now test the actual function
        console.log('=== Testing Actual Function ===');
        const circulatingSupply = await program.getCirculatingSupply({ onycMint: tokenOutMint });
        console.log('Circulating supply type:', circulatingSupply.constructor.name);
        console.log('Circulating supply toString():', circulatingSupply.toString());
        console.log('Circulating supply toString() length:', circulatingSupply.toString().length);
        console.log('Expected:', mintInfo.supply.toString());
        console.log('Match:', circulatingSupply.toString() === mintInfo.supply.toString() ? 'YES' : 'NO');
    });
});
