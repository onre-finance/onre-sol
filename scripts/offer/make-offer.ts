import { TransactionInstruction } from "@solana/web3.js";
import { ScriptHelper, TOKEN_IN_MINT, TOKEN_OUT_MINT } from "../utils/script-helper";

async function createMakeOfferTransaction() {
    const helper = await ScriptHelper.create();

    console.log("Creating make offer transaction...");
    console.log("Token In (USDC):", TOKEN_IN_MINT.toBase58());
    console.log("Token Out (ONe):", TOKEN_OUT_MINT.toBase58());

    const boss = await helper.getBoss();
    console.log("Boss:", boss.toBase58());

    const allowPermissionless = true; // Set to true to enable permissionless

    try {
        const instructions: TransactionInstruction[] = [];

        // If permissionless is enabled, create the intermediary token accounts
        if (allowPermissionless) {
            const permissionlessIxs = await helper.buildCreatePermissionlessTokenAccountsIxs({
                tokenInMint: TOKEN_IN_MINT,
                tokenOutMint: TOKEN_OUT_MINT
            });
            instructions.push(...permissionlessIxs);

            if (permissionlessIxs.length > 0) {
                console.log(`Added ${permissionlessIxs.length} instruction(s) to create permissionless token accounts`);
            } else {
                console.log("Permissionless token accounts already exist");
            }
        }

        // const makeOfferIx = await helper.buildMakeOfferIx({
        //     tokenInMint: TOKEN_IN_MINT,
        //     tokenOutMint: TOKEN_OUT_MINT,
        //     feeBasisPoints: 0, // 0% fee
        //     needsApproval: false, // No approval required
        //     allowPermissionless: allowPermissionless
        // });
        // instructions.push(makeOfferIx);

        const tx = await helper.prepareTransactionMultipleIxs(instructions);

        return helper.printTransaction(tx, "Make Offer Transaction");
    } catch (error) {
        console.error("Error creating transaction:", error);
        throw error;
    }
}

async function main() {
    try {
        await createMakeOfferTransaction();
    } catch (error) {
        console.error("Failed to create make offer transaction:", error);
    }
}

await main();