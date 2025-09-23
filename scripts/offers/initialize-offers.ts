import { ScriptHelper } from '../utils/script-helper';

async function createInitializeOffersTransaction() {
    const helper = await ScriptHelper.create();

    console.log('Creating initialize offers transaction...');

    const boss = await helper.getBoss();
    console.log('Boss:', boss.toBase58());
    console.log('Offers Account PDA:', helper.pdas.offerAccountPda.toBase58());

    try {
        // Check if offers account already exists
        try {
            await helper.getOfferAccount();
            console.log('Offers account already exists!');
            return;
        } catch (error) {
            // Expected - account doesn't exist yet
            console.log('Offers account not found, creating...');
        }

        const tx = await helper.program.methods
            .initializeOffers()
            .accounts({
                state: helper.statePda
            })
            .transaction();

        const preparedTx = await helper.prepareTransaction(tx);
        return helper.printTransaction(preparedTx, 'Initialize Offers Transaction');
    } catch (error) {
        console.error('Error creating transaction:', error);
        throw error;
    }
}

async function main() {
    try {
        await createInitializeOffersTransaction();
    } catch (error) {
        console.error('Failed to create initialize offers transaction:', error);
    }
}

await main();