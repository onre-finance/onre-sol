import chalk from "chalk";
import { Keypair, PublicKey, sendAndConfirmTransaction, SystemProgram, TransactionInstruction } from "@solana/web3.js";
import { select, confirm } from "@inquirer/prompts";
import ora from "ora";
import * as fs from "fs";
import * as os from "os";
import { config } from "../../../utils/script-helper";
import type { GlobalOptions } from "../../prompts";
import { buildAndHandleTransaction, executeCommand } from "../../helpers";
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
            { pubkey: payer, isSigner: true, isWritable: true },
        ],
        data,
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
            BPF_LOADER_UPGRADEABLE_PROGRAM_ID,
        );

        // Fetch current ProgramData account info
        const programDataAccount = await helper.connection.getAccountInfo(programDataPda);
        const currentSize = programDataAccount?.data.length ?? 0;

        if (!opts.json) {
            console.log(chalk.gray("\nProgram extend details:"));
            console.log(`  Program ID:       ${config.programId.toBase58()}`);
            console.log(`  Program Data:     ${programDataPda.toBase58()}`);
            console.log(`  Current size:     ${currentSize.toLocaleString()} bytes`);
            console.log(`  Additional bytes: ${additionalBytes.toLocaleString()}`);
            console.log(`  New size:         ${(currentSize + additionalBytes).toLocaleString()} bytes`);
            console.log();
        }

        // For dry-run/json, default to boss as payer and use standard flow
        if (opts.dryRun || opts.json) {
            await buildAndHandleTransaction(context, {
                buildIx: async () => buildExtendIx(programDataPda, additionalBytes, config.boss),
                title: "Extend Program Data Account",
                description: `Extends program data account by ${additionalBytes.toLocaleString()} bytes`,
                payer: config.boss,
            });
            return;
        }

        // Prompt for signing method — this is permissionless so anyone can pay
        const signingMethod = await select({
            message: "How would you like to sign? (ExtendProgram is permissionless — anyone can pay)",
            choices: [
                { name: "Sign locally with personal wallet", value: "local" },
                { name: "Generate Base58 for Squad multisig", value: "squad" },
            ],
        });

        if (signingMethod === "squad") {
            await buildAndHandleTransaction(context, {
                buildIx: async () => buildExtendIx(programDataPda, additionalBytes, config.boss),
                title: "Extend Program Data Account",
                description: `Extends program data account by ${additionalBytes.toLocaleString()} bytes`,
                payer: config.boss,
            });
            return;
        }

        // Local signing: load keypair first, then build tx with its pubkey as payer
        const keypairPath = `${os.homedir()}/.config/solana/id.json`;
        if (!fs.existsSync(keypairPath)) {
            console.log(chalk.red(`\nKeypair file not found: ${keypairPath}`));
            return;
        }

        let keypair: Keypair;
        try {
            const keypairData = JSON.parse(fs.readFileSync(keypairPath, "utf-8"));
            keypair = Keypair.fromSecretKey(new Uint8Array(keypairData));
        } catch (error) {
            console.log(chalk.red("\nFailed to load keypair:"), error);
            return;
        }

        console.log(chalk.gray(`\nPayer: ${keypair.publicKey.toBase58()}`));

        const confirmed = await confirm({
            message: "Send transaction to chain?",
            default: false,
        });

        if (!confirmed) {
            console.log(chalk.yellow("\nTransaction cancelled."));
            return;
        }

        // Build instruction and transaction with local wallet as payer
        const ix = buildExtendIx(programDataPda, additionalBytes, keypair.publicKey);
        const tx = await helper.prepareTransaction({ ix, payer: keypair.publicKey });

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