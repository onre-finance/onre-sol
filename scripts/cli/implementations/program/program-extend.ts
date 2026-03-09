import chalk from "chalk";
import { PublicKey, sendAndConfirmTransaction, SystemProgram, TransactionInstruction } from "@solana/web3.js";
import { confirm } from "@inquirer/prompts";
import ora from "ora";
import { config } from "../../../utils/script-helper";
import type { GlobalOptions } from "../../prompts";
import { executeCommand } from "../../helpers";
import { extendProgramParams } from "../../params";

const BPF_LOADER_UPGRADEABLE_PROGRAM_ID = new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111");

function buildExtendIx(programDataPda: PublicKey, additionalBytes: number, payer: PublicKey): TransactionInstruction {
    // Instruction layout: u32 instruction index (6) + u32 additional_bytes
    const data = Buffer.alloc(8);
    data.writeUInt32LE(6, 0); // ExtendProgram variant index
    data.writeUInt32LE(additionalBytes, 4);

    return new TransactionInstruction({
        programId: BPF_LOADER_UPGRADEABLE_PROGRAM_ID,
        keys: [
            { pubkey: programDataPda, isSigner: false, isWritable: true },
            { pubkey: config.programId, isSigner: false, isWritable: true },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            { pubkey: payer, isSigner: true, isWritable: true }
        ],
        data
    });
}

/**
 * Execute program extend command
 */
export async function executeProgramExtend(opts: GlobalOptions & Record<string, any>): Promise<void> {
    await executeCommand(opts, extendProgramParams, async (context) => {
        const { helper, params } = context;

        const additionalBytes = Number(params.bytes);

        // Derive ProgramData address
        const [programDataPda] = PublicKey.findProgramAddressSync(
            [config.programId.toBuffer()],
            BPF_LOADER_UPGRADEABLE_PROGRAM_ID
        );

        // Fetch current ProgramData account info
        const programDataAccount = await helper.connection.getAccountInfo(programDataPda);
        const currentSize = programDataAccount?.data.length ?? 0;

        // Calculate additional rent cost
        const rentForCurrent = await helper.connection.getMinimumBalanceForRentExemption(currentSize);
        const rentForNew = await helper.connection.getMinimumBalanceForRentExemption(currentSize + additionalBytes);
        const additionalRentLamports = rentForNew - rentForCurrent;
        const additionalRentSol = additionalRentLamports / 1e9;

        if (!opts.json) {
            console.log(chalk.gray("\nProgram extend details:"));
            console.log(`  Program ID:       ${config.programId.toBase58()}`);
            console.log(`  Program Data:     ${programDataPda.toBase58()}`);
            console.log(`  Current size:     ${currentSize.toLocaleString()} bytes`);
            console.log(`  Additional bytes: ${additionalBytes.toLocaleString()}`);
            console.log(`  New size:         ${(currentSize + additionalBytes).toLocaleString()} bytes`);
            console.log(`  Rent cost:        ${additionalRentSol} SOL`);
            console.log();
        }

        // ExtendProgram is permissionless and cannot be executed via Squads CPI,
        // so we always sign locally with the wallet from Solana CLI config
        const keypair = helper.wallet.payer;
        const ix = buildExtendIx(programDataPda, additionalBytes, keypair.publicKey);
        const tx = await helper.prepareTransaction({ ix, payer: keypair.publicKey });

        if (opts.json) {
            const base58 = helper.serializeTransaction(tx);
            console.log(JSON.stringify({
                title: "Extend Program Data Account",
                base58,
                feePayer: keypair.publicKey.toBase58(),
                additionalBytes,
                rentCostSol: additionalRentSol
            }, null, 2));
            return;
        }

        if (opts.dryRun) {
            const base58 = helper.serializeTransaction(tx);
            console.log(chalk.yellow("[Dry Run] Transaction generated but not executed."));
            console.log(chalk.gray("\nBase58:"));
            console.log(base58);
            return;
        }

        console.log(chalk.gray(`Payer: ${keypair.publicKey.toBase58()} (${helper.walletSource})`));

        const confirmed = await confirm({
            message: "Send transaction to chain?",
            default: false
        });

        if (!confirmed) {
            console.log(chalk.yellow("\nTransaction cancelled."));
            return;
        }

        const spinner = ora("Sending transaction...").start();
        try {
            tx.partialSign(keypair);
            const signature = await sendAndConfirmTransaction(helper.connection, tx, [keypair], { commitment: "confirmed" });

            spinner.succeed(chalk.green("Transaction confirmed!"));
            console.log(chalk.gray(`\nSignature: ${signature}`));

            const network = helper.networkConfig.name;
            const cluster = network.startsWith("devnet") ? "?cluster=devnet" : "";
            console.log(chalk.blue(`Explorer: https://solscan.io/tx/${signature}${cluster}`));
        } catch (error: any) {
            spinner.fail(chalk.red("Transaction failed"));
            console.log(chalk.red("\nError:"), error.message || error);

            if (error.logs) {
                console.log(chalk.gray("\nTransaction logs:"));
                error.logs.forEach((log: string) => console.log(chalk.gray(`  ${log}`)));
            }
        }
    });
}