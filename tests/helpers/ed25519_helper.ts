import { PublicKey, TransactionInstruction, Keypair, Transaction, ComputeBudgetProgram } from "@solana/web3.js";
import { OnreProgram } from "../onre_program";
import BN from "bn.js";
import { sign } from "tweetnacl";

export interface ApprovalMessage {
    programId: PublicKey;
    userPubkey: PublicKey;
    amount: BN;
    expiryUnix: BN;
}

export class Ed25519Helper {
    /**
     * Serializes an approval message for signing
     */
    static serializeApprovalMessage(message: ApprovalMessage): Buffer {
        return Buffer.concat([
            message.programId.toBuffer(),
            message.userPubkey.toBuffer(),
            Buffer.from(message.amount.toArray('le', 8)),
            Buffer.from(message.expiryUnix.toArray('le', 8))
        ]);
    }

    /**
     * Signs an approval message with the given keypair
     */
    static signApprovalMessage(
        message: ApprovalMessage,
        signerKeypair: Keypair
    ): Uint8Array {
        const serializedMessage = this.serializeApprovalMessage(message);

        // Use tweetnacl to sign with the keypair's secret key
        const signature = sign.detached(serializedMessage, signerKeypair.secretKey);

        return signature;
    }

    /**
     * Creates an Ed25519 verification instruction
     */
    static createEd25519Instruction(
        signature: Uint8Array,
        publicKey: PublicKey,
        message: Buffer
    ): TransactionInstruction {
        const ED25519_PROGRAM_ID = new PublicKey("Ed25519SigVerify111111111111111111111111111");

        // Create the instruction data according to Ed25519 program format
        // Header: 1 + 1 + 2 + 2 + 2 + 2 + 2 + 2 + 2 = 16 bytes
        const headerSize = 16;
        const signatureOffset = headerSize;
        const publicKeyOffset = signatureOffset + 64;
        const messageOffset = publicKeyOffset + 32;

        const instructionData = Buffer.concat([
            Buffer.from([1]), // num_signatures (u8)
            Buffer.from([0]), // padding (u8)

            // Signature info
            Buffer.from([signatureOffset & 0xFF, (signatureOffset >> 8) & 0xFF]), // signature_offset (u16 LE)
            Buffer.from([0, 0]), // signature_instruction_index (u16 LE) - 0 means current instruction

            // Public key info
            Buffer.from([publicKeyOffset & 0xFF, (publicKeyOffset >> 8) & 0xFF]), // public_key_offset (u16 LE)
            Buffer.from([0, 0]), // public_key_instruction_index (u16 LE) - 0 means current instruction

            // Message info
            Buffer.from([messageOffset & 0xFF, (messageOffset >> 8) & 0xFF]), // message_data_offset (u16 LE)
            Buffer.from([message.length & 0xFF, (message.length >> 8) & 0xFF]), // message_data_size (u16 LE)
            Buffer.from([0, 0]), // message_instruction_index (u16 LE) - 0 means current instruction

            // Data
            Buffer.from(signature), // signature (64 bytes)
            publicKey.toBuffer(), // public key (32 bytes)
            message // message (variable length)
        ]);

        return new TransactionInstruction({
            programId: ED25519_PROGRAM_ID,
            keys: [],
            data: instructionData
        });
    }

    /**
     * Creates a complete Ed25519 instruction for approval message verification
     */
    static createApprovalInstruction(
        message: ApprovalMessage,
        signerKeypair: Keypair
    ): TransactionInstruction {
        const serializedMessage = this.serializeApprovalMessage(message);
        const signature = this.signApprovalMessage(message, signerKeypair);

        return this.createEd25519Instruction(
            signature,
            signerKeypair.publicKey,
            serializedMessage
        );
    }

    /**
     * Helper to execute take offer with approval in a single transaction
     */
    static async executeApprovedTakeOffer(params: {
        program: OnreProgram;
        tokenInAmount: number;
        tokenInMint: PublicKey;
        tokenOutMint: PublicKey;
        user: PublicKey;
        userKeypair: Keypair;
        trustedAuthority: Keypair;
        boss: PublicKey;
        expiryTime?: number;
    }) {
        const expiryTime = params.expiryTime || (Math.floor(Date.now() / 1000) + 3600);

        // Create approval message
        const approvalMessage: ApprovalMessage = {
            programId: params.program.program.programId,
            userPubkey: params.user,
            amount: new BN(params.tokenInAmount),
            expiryUnix: new BN(expiryTime)
        };

        // Create Ed25519 verification instruction
        const ed25519Instruction = this.createApprovalInstruction(
            approvalMessage,
            params.trustedAuthority
        );

        // Create transaction with Ed25519 verification and take offer
        const tx = params.program.program.methods
            .takeOffer(new BN(params.tokenInAmount), approvalMessage)
            .accounts({
                state: params.program.statePda,
                boss: params.boss,
                tokenInMint: params.tokenInMint,
                tokenOutMint: params.tokenOutMint,
                user: params.user,
                tokenInProgram: new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
                tokenOutProgram: new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
            })
            .preInstructions([
                ComputeBudgetProgram.setComputeUnitLimit({ units: 300_000 }),
                ed25519Instruction
            ])
            .signers([params.userKeypair]);

        await tx.rpc();
    }
}