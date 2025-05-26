import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { evmAddressToBytes32, getAnchorConnection, getDepositForBurnPdas, getAttestation, getPrograms, ETH_RPC_URL, SOL_RPC_URL, SOLANA_USDC_ADDRESS, ETH_DOMAIN_ID, ETH_MESSAGE_TRANSMITTER_ADDRESS, SOLANA_DOMAIN_ID } from "./utils.ts";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes/index.js";
import { Contract, getBytes, JsonRpcProvider, Wallet } from "ethers";
import messageTransmitterAbi from "./abis/MessageTransmitter.json" with { type: "json" }
import anchor from "@coral-xyz/anchor";
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";

// SCRIPT PARAMETERS
const SOLANA_SENDER_PRIVATE_KEY = "INSERT PRIVATE KEY HERE";
const ETH_RECIPIENT_ADDRESS = "INSERT ETHEREUM RECIPIENT ADDRESS HERE";
const ETH_PAYER_PRIVATE_KEY = "INSERT PRIVATE KEY HERE";
const USDC_AMOUNT = 10000; // $0.01


const ethereumProvider = new JsonRpcProvider(ETH_RPC_URL);
const solanaProvider = getAnchorConnection(SOL_RPC_URL, Keypair.fromSecretKey(bs58.decode(SOLANA_SENDER_PRIVATE_KEY)));

const solanaSenderTokenAccount = await getAssociatedTokenAddress(
    new PublicKey(SOLANA_USDC_ADDRESS),
    solanaProvider.wallet.publicKey
);

async function burnUSDC() {
    const { messageTransmitterProgram, tokenMessengerMinterProgram } = getPrograms(solanaProvider);

    const mintRecipient = new PublicKey(getBytes(evmAddressToBytes32(ETH_RECIPIENT_ADDRESS)));

    // Get pdas
    const pdas = getDepositForBurnPdas({messageTransmitterProgram, tokenMessengerMinterProgram}, new PublicKey(SOLANA_USDC_ADDRESS), ETH_DOMAIN_ID);

    // Generate a new keypair for the MessageSent event account.
    const messageSentEventAccountKeypair = Keypair.generate();
    
    // Call depositForBurn
    const depositForBurnTx = await tokenMessengerMinterProgram.methods
    .depositForBurn({
        amount: new anchor.BN(USDC_AMOUNT),
        destinationDomain: ETH_DOMAIN_ID,
        mintRecipient,
    })
    // eventAuthority and program accounts are implicitly added by Anchor 
    .accountsPartial({
        owner: solanaProvider.wallet.publicKey,
        eventRentPayer: solanaProvider.wallet.publicKey,
        senderAuthorityPda: pdas.authorityPda.publicKey,
        localToken: pdas.localToken.publicKey,
        messageTransmitter: pdas.messageTransmitterAccount.publicKey,
        tokenMessenger: pdas.tokenMessengerAccount.publicKey,
        remoteTokenMessenger: pdas.remoteTokenMessengerKey.publicKey,
        tokenMinter: pdas.tokenMinterAccount.publicKey,
        burnTokenMint: new PublicKey(SOLANA_USDC_ADDRESS),
        burnTokenAccount: solanaSenderTokenAccount,
        messageTransmitterProgram: messageTransmitterProgram.programId,
        tokenMessengerMinterProgram: tokenMessengerMinterProgram.programId,
        messageSentEventData: messageSentEventAccountKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
    })
    // messageSentEventAccountKeypair must be a signer so the MessageTransmitter program can take control of it and write to it.
    // provider.wallet is also an implicit signer
    .signers([messageSentEventAccountKeypair])
    .rpc({
        commitment: "confirmed",
        maxRetries: 5,
        preflightCommitment: "confirmed",
        skipPreflight: false,
    });

    console.log("DepositForBurn transaction sent:", depositForBurnTx);

    return depositForBurnTx;
}

async function mintUSDC(message: string, attestation: string) {
    const messageTransmitter = new Contract(ETH_MESSAGE_TRANSMITTER_ADDRESS, messageTransmitterAbi, new Wallet(ETH_PAYER_PRIVATE_KEY, ethereumProvider));

    try {
        const tx = await messageTransmitter.receiveMessage(message, attestation);
        console.log("Received message on Ethereum. Tx:", tx.hash);
        return tx.hash;
    } catch (error: any) {
        console.error("Error in mintUSDC:");
        console.error("Error message:", error.message);
    }
}

async function transferUsdc() {
    // STEP 1: Burn USDC
    const txHash = await burnUSDC();

    // STEP 2: Get attestation
    const { message, attestation } = await getAttestation(txHash, SOLANA_DOMAIN_ID, 40, 2000);

    // STEP 3: Mint USDC on Ethereum
    await mintUSDC(message, attestation);
}

transferUsdc();