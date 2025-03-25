import * as anchor from '@coral-xyz/anchor';
import {AnchorProvider} from '@coral-xyz/anchor';
import {PublicKey, Transaction} from '@solana/web3.js';
import {
    createAssociatedTokenAccountInstruction,
    createTransferInstruction,
    getAssociatedTokenAddressSync
} from '@solana/spl-token'
import bs58 from 'bs58';
const PROGRAM_ID = new PublicKey('J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2');

const BOSS = new PublicKey('7rzEKejyAXJXMkGfRhMV9Vg1k7tFznBBEFu3sfLNz8LC');

const TOKEN_MINT = new PublicKey("qaegW5BccnepuexbHkVqcqQUuEwgDMqCCo1wJ4fWeQu");

const RECIPIENT = new PublicKey("9tTUg7r9ftofzoPXKeUPB35oN4Lm8KkrVDVQbbM7Xzxx")


async function createMakeOfferOneTransaction() {
    const connection = new anchor.web3.Connection('https://api.mainnet-beta.solana.com');
    const wallet = new anchor.Wallet(anchor.web3.Keypair.generate());
    const provider = new AnchorProvider(connection, wallet);
    anchor.setProvider(provider);

    // Only do this if the associated token accounts don't exist yet, otherwise skip
    const createRecipientATAInstruction = createAssociatedTokenAccountInstruction(
        BOSS,
        getAssociatedTokenAddressSync(TOKEN_MINT, RECIPIENT),
        RECIPIENT,
        TOKEN_MINT
    );

    const amount = 100e9

    try {
        const transferInstruction = createTransferInstruction(
            getAssociatedTokenAddressSync(TOKEN_MINT, BOSS, true),
            getAssociatedTokenAddressSync(TOKEN_MINT, RECIPIENT),
            BOSS,
            amount
        )

        const tx = new Transaction();
        tx.add(createRecipientATAInstruction, transferInstruction);

        tx.feePayer = BOSS;
        tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

        const serializedTx = tx.serialize({
            requireAllSignatures: false,
            verifySignatures: false,
        });

        const base58Tx = bs58.encode(serializedTx);
        console.log('Make Offer Transaction (Base58):');
        console.log(base58Tx);

        return base58Tx;
    } catch (error) {
        console.error('Error creating transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createMakeOfferOneTransaction();
    } catch (error) {
        console.error('Failed to create make offer transaction:', error);
    }
}

main();