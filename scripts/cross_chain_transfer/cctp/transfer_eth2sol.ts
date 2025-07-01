import { bs58 } from '@coral-xyz/anchor/dist/cjs/utils/bytes/index.js';
import tokenMessengerAbi from './abis/TokenMessenger.json' with { type: "json" };
import usdcAbi from './abis/Usdc.json' with { type: "json" };
import { JsonRpcProvider, Wallet, Contract, hexlify, keccak256, ContractTransactionResponse, ethers } from 'ethers';
import { decodeEventNonceFromMessage, ETH_DOMAIN_ID, ETH_RPC_URL, ETH_TOKEN_MESSENGER_ADDRESS, ETH_USDC_ADDRESS, getAnchorConnection, getAttestation, getPrograms, getReceiveMessagePdas, SOL_RPC_URL, SOLANA_DOMAIN_ID, SOLANA_USDC_ADDRESS, solanaAddressToHex } from './utils.ts';
import { Keypair, PublicKey, SystemProgram, Transaction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddress, createAssociatedTokenAccountInstruction } from '@solana/spl-token';

// SCRIPT PARAMETERS
const ETH_SENDER_PRIVATE_KEY = "INSERT PRIVATE KEY HERE";
const SOLANA_RECIPIENT_ADDRESS = "INSERT SOLANA RECIPIENT ADDRESS HERE";
const SOLANA_PAYER_PRIVATE_KEY = "INSERT PRIVATE KEY HERE"; // pays for the creation of the ATA
const USDC_AMOUNT = BigInt(100000); // $0.01

const ethereumProvider = new JsonRpcProvider(ETH_RPC_URL);
const solanaProvider = getAnchorConnection(SOL_RPC_URL, Keypair.fromSecretKey(bs58.decode(SOLANA_PAYER_PRIVATE_KEY)));

const ethSigner = new Wallet(ETH_SENDER_PRIVATE_KEY, ethereumProvider);
const solanaRecipientTokenAccount = await getAssociatedTokenAddress(
    new PublicKey(SOLANA_USDC_ADDRESS),
    new PublicKey(SOLANA_RECIPIENT_ADDRESS)
);

async function burnUSDC() {
    const usdc = new Contract(ETH_USDC_ADDRESS, usdcAbi, ethSigner);
    
    // Get the current nonce
    const nonce = await ethereumProvider.getTransactionCount(ethSigner.address, "latest");
    
    // Approve USDC for TokenMessenger
    const tx = await usdc.approve(ETH_TOKEN_MESSENGER_ADDRESS, USDC_AMOUNT, {
        nonce: nonce
    });
    await tx.wait();
    console.log("USDC transfer approved for TokenMessenger, txHash:", tx.hash);

    const tokenMessenger = new Contract(ETH_TOKEN_MESSENGER_ADDRESS, tokenMessengerAbi, ethSigner);
    
    // Check USDC balance and allowance
    const balance = await usdc.balanceOf(ethSigner.address);
    const allowance = await usdc.allowance(ethSigner.address, ETH_TOKEN_MESSENGER_ADDRESS);
    
    if (allowance < USDC_AMOUNT) {
        throw new Error(`Insufficient allowance. Current: ${allowance}, Required: ${USDC_AMOUNT}`);
    }
    
    if (balance < USDC_AMOUNT) {
        throw new Error(`Insufficient balance. Current: ${balance}, Required: ${USDC_AMOUNT}`);
    }

    // Convert Solana address to bytes32 format
    const solanaRecipientTokenAccountHex = solanaAddressToHex(solanaRecipientTokenAccount.toBase58());

    // Check if the token account exists
    const tokenAccountInfo = await solanaProvider.connection.getAccountInfo(solanaRecipientTokenAccount);
    
    // If the token account doesn't exist, create it
    if (!tokenAccountInfo) {
        console.log("Creating associated token account for recipient...");
        const createAtaIx = createAssociatedTokenAccountInstruction(
            solanaProvider.wallet.publicKey, // payer
            solanaRecipientTokenAccount, // ata
            new PublicKey(SOLANA_RECIPIENT_ADDRESS), // owner
            new PublicKey(SOLANA_USDC_ADDRESS) // mint
        );
        
        const { blockhash } = await solanaProvider.connection.getLatestBlockhash('confirmed');
        const transaction = new Transaction().add(createAtaIx);
        transaction.recentBlockhash = blockhash;
        transaction.feePayer = solanaProvider.wallet.publicKey;
        
        const tx = await solanaProvider.sendAndConfirm(transaction, [Keypair.fromSecretKey(bs58.decode(SOLANA_PAYER_PRIVATE_KEY))],
         {
            commitment: 'confirmed',
         });

        console.log("Created ATA transaction:", tx);
    }
    
    try {
        // Get the current nonce
        const nonce = await ethereumProvider.getTransactionCount(ethSigner.address, "latest");

        const tx: ContractTransactionResponse = await tokenMessenger.depositForBurn(
            USDC_AMOUNT,
            SOLANA_DOMAIN_ID,
            solanaRecipientTokenAccountHex,
            ETH_USDC_ADDRESS,
            {
                nonce: nonce
            }
        );
        console.log("DepositForBurn transaction sent:", tx.hash);
        await tx.wait();
        return tx.hash;
    } catch (error: any) {
        console.error("Error in burnUSDC:");
        console.error("Error message:", error.message);
        if (error.data) {
            console.error("Error data:", error.data);
        }
        if (error.transaction) {
            console.error("Transaction:", error.transaction);
        }
        throw error;
    }
}

async function mintUSDC(messageHex: string, attestationHex: string) {
    const { messageTransmitterProgram, tokenMessengerMinterProgram } = getPrograms(solanaProvider);

    // Get PDAs
    const pdas = await getReceiveMessagePdas(
        {messageTransmitterProgram, tokenMessengerMinterProgram},
        new PublicKey(SOLANA_USDC_ADDRESS),
        ETH_USDC_ADDRESS,
        ETH_DOMAIN_ID.toString(),
        decodeEventNonceFromMessage(messageHex),
    )

    // accountMetas list to pass to remainingAccounts
    const accountMetas: any[] = [];
    accountMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: pdas.tokenMessengerAccount.publicKey,
    });
    accountMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: pdas.remoteTokenMessengerKey.publicKey,
    });
    accountMetas.push({
        isSigner: false,
        isWritable: true,
        pubkey: pdas.tokenMinterAccount.publicKey,
    });
    accountMetas.push({
        isSigner: false,
        isWritable: true,
        pubkey: pdas.localToken.publicKey,
    });
    accountMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: pdas.tokenPair.publicKey,
    });
    accountMetas.push({
        isSigner: false,
        isWritable: true,
        pubkey: solanaRecipientTokenAccount,
    });
    accountMetas.push({
        isSigner: false,
        isWritable: true,
        pubkey: pdas.custodyTokenAccount.publicKey,
    });
    accountMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: TOKEN_PROGRAM_ID,
    });
    accountMetas.push({
      isSigner: false,
      isWritable: false,
      pubkey: pdas.tokenMessengerEventAuthority.publicKey,
    });
    accountMetas.push({
      isSigner: false,
      isWritable: false,
      pubkey: tokenMessengerMinterProgram.programId,
    });

    try {
        const receiveMessageTx = await messageTransmitterProgram.methods
            .receiveMessage({
                message: Buffer.from(messageHex.replace("0x", ""), "hex"),
                attestation: Buffer.from(attestationHex.replace("0x", ""), "hex"),
            })
            .accountsPartial({
                payer: solanaProvider.wallet.publicKey,
                caller: solanaProvider.wallet.publicKey,
                authorityPda: pdas.authorityPda,
                messageTransmitter: pdas.messageTransmitterAccount.publicKey,
                usedNonces: pdas.usedNonces,
                receiver: tokenMessengerMinterProgram.programId,
                systemProgram: SystemProgram.programId,
            })
            .remainingAccounts(accountMetas)
            .rpc({
                commitment: "confirmed",
                maxRetries: 5,
                preflightCommitment: "confirmed",
                skipPreflight: false,
            });
        
        console.log("ReceiveMessage transaction sent:", receiveMessageTx);
    } catch (error: any) {
        console.error("Error in mintUSDC:");
        console.error("Error message:", error.message);
        if (error.logs) {
            console.error("Transaction logs:", error.logs);
        }
        throw error;
    }
}

async function transferUsdc() {
    // STEP 1: Burn USDC
    //const txHash = await burnUSDC();

    // STEP 2: Get message and hash
    const { message, attestation } = await getAttestation('0x40300b77ecd2c2f3611a23fd653e25d964ac6734b7ffc3a2fdeb6b4d981ff529', ETH_DOMAIN_ID, 40, 30000);

    // STEP 3: Mint USDC on Solana
    await mintUSDC(message, attestation);
}

transferUsdc();