// scripts/fetchOffer.ts
import * as anchor from '@coral-xyz/anchor';
import { AnchorProvider, Program } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import type { OnreApp } from '../target/types/onre_app';
import { BN } from 'bn.js';

import idl from "../target/idl/onre_app.json" assert {type: "json"};

const PROGRAM_ID = new PublicKey('J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2');

async function fetchOffer(offerId: BN) {
    const connection = new anchor.web3.Connection('https://api.mainnet-beta.solana.com');
    const wallet = new anchor.Wallet(anchor.web3.Keypair.generate());
    const provider = new AnchorProvider(connection, wallet);
    const program = new Program(idl as OnreApp, provider);
    anchor.setProvider(provider);

    const [offerPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('offer'), offerId.toArrayLike(Buffer, 'le', 8)],
        PROGRAM_ID
    );

    try {
        // Fetch the offer account data
        const offerAccount = await program.account.offer.fetch(offerPda);

        // Log the offer details
        console.log('Offer Account Data:');
        console.log('  Offer ID:', offerAccount.offerId.toString());
        console.log('  Sell Token Mint:', offerAccount.sellTokenMint.toString());
        console.log('  Buy Token Mint 1:', offerAccount.buyTokenMint1.toString());
        console.log('  Buy Token Mint 2:', offerAccount.buyTokenMint2.toString());
        console.log('  Buy Token 1 Total Amount:', offerAccount.buyToken1TotalAmount.toString());
        console.log('  Buy Token 2 Total Amount:', offerAccount.buyToken2TotalAmount.toString());
        console.log('  Sell Token Total Amount:', offerAccount.sellTokenTotalAmount.toString());
        console.log('  Authority Bump:', offerAccount.authorityBump);

        return offerAccount;
    } catch (error) {
        console.error('Error fetching offer:', error);
        throw error;
    }
}

async function main() {
    try {
        const offerId = new BN(3); // Replace with the desired offer ID
        await fetchOffer(offerId);
    } catch (error) {
        console.error('Failed to fetch offer:', error);
    }
}

main();