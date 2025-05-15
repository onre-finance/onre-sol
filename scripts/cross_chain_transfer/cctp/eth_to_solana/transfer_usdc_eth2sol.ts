import { bs58 } from '@coral-xyz/anchor/dist/cjs/utils/bytes/index.js';
import tokenMessengerAbi from '../abis/TokenMessenger.json' with { type: "json" };
import usdcAbi from '../abis/Usdc.json' with { type: "json" };
import { JsonRpcProvider, Wallet, Contract, hexlify, keccak256, ContractTransactionResponse, ethers } from 'ethers';
import { decodeEventNonceFromMessage, getAnchorConnection, getPrograms, getReceiveMessagePdas } from '../utils/utils.ts';
import { Keypair, PublicKey, SystemProgram, Transaction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddress, createAssociatedTokenAccountInstruction } from '@solana/spl-token';

// SCRIPT PARAMETERS
const ETH_SENDER_PRIVATE_KEY = "INSERT ETH PRIVATE KEY HERE";
const SOLANA_RECIPIENT_ADDRESS = "INSERT SOLANA RECIPIENT ADDRESS HERE";
const SOLANA_PAYER_PRIVATE_KEY = "INSERT SOLANA PRIVATE KEY HERE"; // pays for the creation of the ATA
const USDC_AMOUNT = BigInt(10000); // $0.01

// CONFIGURATION VALUES
const IRIS_API_URL = "https://iris-api.circle.com"
const ETH_RPC_URL = "https://eth.drpc.org";
const SOL_RPC_URL = "https://api.mainnet-beta.solana.com";

const ETH_TOKEN_MESSENGER_ADDRESS = "0xBd3fa81B58Ba92a82136038B25aDec7066af3155";

const ETH_USDC_ADDRESS = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const SOL_USDC_ADDRESS = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

const SOURCE_DOMAIN = 0; // ETHEREUM
const DESTINATION_DOMAIN = 5; // SOLANA

const ethereumProvider = new JsonRpcProvider(ETH_RPC_URL);
const solanaProvider = getAnchorConnection(SOL_RPC_URL, Keypair.fromSecretKey(bs58.decode(SOLANA_PAYER_PRIVATE_KEY)));

const ethSigner = new Wallet(ETH_SENDER_PRIVATE_KEY, ethereumProvider);
const solanaRecipientTokenAccount = await getAssociatedTokenAddress(
    new PublicKey(SOL_USDC_ADDRESS),
    new PublicKey(SOLANA_RECIPIENT_ADDRESS)
);

async function approveUSDC() {
    const usdc = new Contract(ETH_USDC_ADDRESS, usdcAbi, ethSigner);
    
    // Get the current nonce
    const nonce = await ethereumProvider.getTransactionCount(ethSigner.address, "latest");
    console.log("Current nonce for approval:", nonce);
    
    const tx = await usdc.approve(ETH_TOKEN_MESSENGER_ADDRESS, USDC_AMOUNT, {
        nonce: nonce
    });
    console.log("Approval transaction sent:", tx.hash);
    await tx.wait();
    console.log("USDC approved for TokenMessenger");
}

async function burnUSDC() {
    const tokenMessenger = new Contract(ETH_TOKEN_MESSENGER_ADDRESS, tokenMessengerAbi, ethSigner);
    
    // Check USDC balance first
    const usdc = new Contract(ETH_USDC_ADDRESS, usdcAbi, ethSigner);
    const balance = await usdc.balanceOf(ethSigner.address);
    console.log("USDC Balance:", balance.toString());
    
    // Check allowance
    const allowance = await usdc.allowance(ethSigner.address, ETH_TOKEN_MESSENGER_ADDRESS);
    console.log("USDC Allowance:", allowance.toString());
    
    if (allowance < USDC_AMOUNT) {
        throw new Error(`Insufficient allowance. Current: ${allowance}, Required: ${USDC_AMOUNT}`);
    }
    
    if (balance < USDC_AMOUNT) {
        throw new Error(`Insufficient balance. Current: ${balance}, Required: ${USDC_AMOUNT}`);
    }

    // Convert Solana address to bytes32 format
    const decodedAddress = bs58.decode(solanaRecipientTokenAccount.toBase58());
    const paddedAddress = new Uint8Array(32);
    paddedAddress.set(decodedAddress);
    const solanaRecipientTokenAccountHex = hexlify(paddedAddress);

    const provider = getAnchorConnection(SOL_RPC_URL, Keypair.fromSecretKey(bs58.decode(SOLANA_PAYER_PRIVATE_KEY)));

    // Check if the token account exists
    const tokenAccountInfo = await provider.connection.getAccountInfo(solanaRecipientTokenAccount);
    
    // If the token account doesn't exist, create it
    if (!tokenAccountInfo) {
        console.log("Creating associated token account for recipient...");
        const createAtaIx = createAssociatedTokenAccountInstruction(
            provider.wallet.publicKey, // payer
            solanaRecipientTokenAccount, // ata
            new PublicKey(SOLANA_RECIPIENT_ADDRESS), // owner
            new PublicKey(SOL_USDC_ADDRESS) // mint
        );
        
        const { blockhash } = await provider.connection.getLatestBlockhash('finalized');
        const transaction = new Transaction().add(createAtaIx);
        transaction.recentBlockhash = blockhash;
        transaction.feePayer = provider.wallet.publicKey;
        
        const tx = await provider.sendAndConfirm(transaction, [Keypair.fromSecretKey(bs58.decode(SOLANA_PAYER_PRIVATE_KEY))]);
        console.log("Created ATA transaction:", tx);
    }
    
    console.log("Recipient token account:", solanaRecipientTokenAccount.toBase58());
    console.log("Encoded recipient token account:", solanaRecipientTokenAccountHex);
    console.log("Amount:", USDC_AMOUNT.toString());
    console.log("Destination Domain:", DESTINATION_DOMAIN);
    
    try {
        // Get the current nonce
        const nonce = await ethereumProvider.getTransactionCount(ethSigner.address, "latest");
        console.log("Current nonce:", nonce);

        const tx: ContractTransactionResponse = await tokenMessenger.depositForBurn(
            USDC_AMOUNT,
            DESTINATION_DOMAIN,
            solanaRecipientTokenAccountHex,
            ETH_USDC_ADDRESS,
            {
                nonce: nonce
            }
        );
        console.log("Burning USDC on Ethereum. Tx:", tx.hash);
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

async function fetchAttestation(messageHash: string) {
    // Wait for attestation to be available via IRIS API
    let attestation;
    for (let i = 0; i < 40; i++) {
        console.log(`[Attempt: ${i}] Fetching attestation for ${messageHash} ...`);
        const res = await fetch(`${IRIS_API_URL}/v1/attestations/${messageHash}`);
        const data = await res.json();
        if (data && data.status === "complete" && data.attestation) {
            attestation = data.attestation;
            break;
        }
        // Wait 30 seconds to avoid getting rate limited
        // Creating attestation may take up to 20 minutes
        await new Promise(r => setTimeout(r, 30000));
    }
    if (!attestation) {
        throw new Error("Attestation not found in IRIS API")
    } else {
        console.log("Attestation: ", attestation);
    }
    return attestation;
}

async function getMessageHashFromLogs(txHash: string) {
    const transactionReceipt = await ethereumProvider.getTransactionReceipt(txHash);

    const eventTopic = ethers.id('MessageSent(bytes)')

    const messageSentEvent = transactionReceipt?.logs.find((l) => l.topics[0] === eventTopic)

    if (!messageSentEvent) throw new Error("MessageSent event not found");

    const coder = new ethers.AbiCoder()
    const messageBytes = coder.decode(['bytes'], messageSentEvent.data)[0]
    const messageHash = keccak256(messageBytes)

    console.log("Message Bytes:", messageBytes);
    console.log("Message Hash:", messageHash);

    return {messageBytes, messageHash};
}

async function mintUSDC(messageHex: string, attestationHex: string) {
    const { messageTransmitterProgram, tokenMessengerMinterProgram } = getPrograms(solanaProvider);

    // Get PDAs
    const pdas = await getReceiveMessagePdas(
        {messageTransmitterProgram, tokenMessengerMinterProgram},
        new PublicKey(SOL_USDC_ADDRESS),
        ETH_USDC_ADDRESS,
        SOURCE_DOMAIN.toString(),
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
        // Get a fresh blockhash
        const { blockhash } = await solanaProvider.connection.getLatestBlockhash('finalized');
        console.log("Using blockhash:", blockhash);

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
            });
        
        console.log("\n\nreceiveMessage Tx: ", receiveMessageTx);
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
    // STEP 1: Approve messenger contract to withdraw from our active eth address
    await approveUSDC();

    // STEP 2: Burn USDC
    const txHash = await burnUSDC();

    // STEP 3: Get message and hash
    const {messageBytes, messageHash} = await getMessageHashFromLogs(txHash);

    // STEP 4: Fetch attestation
    const attestation = await fetchAttestation(messageHash);

    // STEP 5: Mint USDC on Solana
    await mintUSDC(messageBytes, attestation);
}

transferUsdc();