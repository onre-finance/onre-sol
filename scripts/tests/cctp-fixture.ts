// Preload only for dependency compatibility tests. Real SDKs construct and sign
// transactions; external sends and Circle attestations stop at fixture boundaries.
import assert from "node:assert/strict";
import { mock } from "node:test";
import anchor from "@anchor-lang/core";
import * as ethers from "ethers";
import bs58 from "bs58";
import { Connection, Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import tokenMessengerIdl from "../cross_chain_transfer/cctp_v2/idl/token_messenger_minter_v2.json";

const wallet = Keypair.fromSeed(new Uint8Array(32).fill(7));
const mint = Keypair.fromSeed(new Uint8Array(32).fill(8)).publicKey;
const tokenAccount = Keypair.fromSeed(new Uint8Array(32).fill(9)).publicKey;
const ethAddress = "0x1111111111111111111111111111111111111111";
const message = Buffer.alloc(160);
message.writeBigUInt64BE(42n, 12);
const attestation = { message: `0x${message.toString("hex")}`, attestation: "0x1234" };
const calls: { chain: string; method: string; data: string; accounts?: string[] }[] = [];

Object.assign(process.env, {
    SOL_DEVNET_RPC_URL: "http://127.0.0.1:1",
    ETH_RPC_URL: "http://127.0.0.1:1",
    SOL_SENDER_PRIVATE_KEY: bs58.encode(wallet.secretKey),
    ETH_SENDER_PRIVATE_KEY: `0x${"07".repeat(32)}`,
    ETH_PAYER_PRIVATE_KEY: `0x${"07".repeat(32)}`,
    SOL_RECIPIENT_ADDRESS: wallet.publicKey.toBase58(),
    ETH_RECIPIENT_ADDRESS: ethAddress,
    SOL_USDC_ADDRESS: mint.toBase58(),
    ETH_USDC_ADDRESS: ethAddress,
    USER_TOKEN_ACCOUNT: tokenAccount.toBase58(),
    USDC_AMOUNT: "1000000",
    ETH_TOKEN_MESSENGER_ADDRESS: ethAddress,
    ETH_MESSAGE_TRANSMITTER_ADDRESS: ethAddress,
    ETH_TOKEN_MESSENGER_ADDRESS_V2: ethAddress,
    ETH_MESSAGE_TRANSMITTER_ADDRESS_V2: ethAddress,
});
delete process.env.DESTINATION_CALLER;

const messengerCoder = new anchor.Program(tokenMessengerIdl as anchor.Idl, new anchor.AnchorProvider(new Connection("http://127.0.0.1:1"), new anchor.Wallet(wallet))).coder.accounts;
const messengerData = await messengerCoder.encode("tokenMessenger", {
    denylister: wallet.publicKey,
    owner: wallet.publicKey,
    pendingOwner: PublicKey.default,
    messageBodyVersion: 0,
    authorityBump: 1,
    feeRecipient: wallet.publicKey,
    minFeeController: wallet.publicKey,
    minFee: 0,
});

Connection.prototype.getAccountInfo = async function (key) {
    return {
        data: messengerData,
        owner: key.equals(tokenAccount) ? TOKEN_PROGRAM_ID : new PublicKey(tokenMessengerIdl.address),
        lamports: 1_000_000_000,
        executable: false,
        rentEpoch: 0,
    };
};
Connection.prototype.getAccountInfoAndContext = async function (key) {
    return { context: { slot: 1 }, value: await this.getAccountInfo(key) };
};
Connection.prototype.getLatestBlockhash = async () => ({ blockhash: PublicKey.default.toBase58(), lastValidBlockHeight: 100 });
anchor.AnchorProvider.prototype.sendAndConfirm = async function (tx, signers = []) {
    assert.ok(tx instanceof Transaction);
    tx.feePayer = this.wallet.publicKey;
    tx.recentBlockhash = PublicKey.default.toBase58();
    await this.wallet.signTransaction(tx);
    if (signers.length) tx.partialSign(...signers);
    assert.ok(tx.verifySignatures(), "SDK-built transaction must have valid signatures");
    for (const ix of tx.instructions) {
        assert.ok(ix.keys.length > 0);
        assert.ok(ix.data.length >= 8);
        calls.push({ chain: "solana", method: ix.programId.toBase58(), data: ix.data.toString("hex"), accounts: ix.keys.map((k) => k.pubkey.toBase58()) });
    }
    return bs58.encode(new Uint8Array(64).fill(3));
};
anchor.AnchorProvider.prototype.simulate = async function (tx) {
    assert.ok(tx instanceof Transaction);
    const programId = tx.instructions[0].programId;
    return { logs: [`Program ${programId} invoke [1]`, `Program return: ${programId} ${tokenAccount.toBuffer().toString("base64")}`, `Program ${programId} success`], unitsConsumed: 100 };
};

ethers.JsonRpcProvider.prototype.getTransactionCount = async () => 0;
class FixtureContract {
    constructor(address: string, abi: ethers.InterfaceAbi, runner: ethers.ContractRunner) {
        const contract = new ethers.Contract(address, abi, runner);
        return new Proxy(this, {
            get: (_target, method) => async (...args: unknown[]) => {
                if (method === "balanceOf" || method === "allowance") return 1_000_000_000n;
                const transaction = await contract.getFunction(String(method)).populateTransaction(...args);
                assert.ok(transaction.data);
                const decoded = contract.interface.parseTransaction({ data: transaction.data });
                assert.equal(decoded?.name, method);
                calls.push({ chain: "evm", method: String(method), data: transaction.data });
                return { hash: `0x${"03".repeat(32)}`, wait: async () => ({ status: 1, hash: `0x${"03".repeat(32)}` }) };
            },
        });
    }
}
mock.module("ethers", { namedExports: { ...ethers, Contract: FixtureContract, ethers: { ...ethers.ethers, Contract: FixtureContract } } });
globalThis.fetch = async () => new Response(JSON.stringify({ messages: [attestation] }), { headers: { "content-type": "application/json" } });

process.on("beforeExit", () => console.log(`COMPAT_CALLS=${JSON.stringify(calls)}`));
