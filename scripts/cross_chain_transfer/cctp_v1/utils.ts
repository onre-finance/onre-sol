/*
 * Copyright (c) 2024, Circle Internet Financial LTD All rights reserved.
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

import 'dotenv/config';
import { PublicKey, Keypair, Connection } from '@solana/web3.js';
import { hexlify } from 'ethers';
import fetch from 'node-fetch';
import { bs58 } from '@coral-xyz/anchor/dist/cjs/utils/bytes/index.js';

import type { MessageTransmitter } from './types/message_transmitter.ts';
import type { TokenMessengerMinter } from './types/token_messenger_minter.ts';
import messageTransmitterIdl from './idl/message_transmitter.json' with { type: "json" };
import tokenMessengerMinterIdl from './idl/token_messenger_minter.json' with { type: "json" };
import anchor, { Program } from "@coral-xyz/anchor";
import { encode } from '@coral-xyz/anchor/dist/cjs/utils/bytes/utf8.js';

// IRIS API
export const IRIS_API_URL = "https://iris-api.circle.com";

// SOLANA constants
export const SOLANA_DOMAIN_ID = 5;

// ETHEREUM constants
export const ETH_DOMAIN_ID = 0;

export interface FindProgramAddressResponse {
    publicKey: PublicKey;
    bump: number;
}

// Configure client to use the provider and return it.
export const getAnchorConnection = () => {
    // Create the provider
    const connection = new Connection(process.env.SOL_RPC_URL!, "confirmed");
    const provider = new anchor.AnchorProvider(
        connection,
        new anchor.Wallet(Keypair.fromSecretKey(bs58.decode(process.env.SOL_SENDER_PRIVATE_KEY!))),
        { commitment: "confirmed" }
    );

    anchor.setProvider(provider);
    return provider;
};

export const getPrograms = (provider: anchor.AnchorProvider) => {
    // Initialize contracts
    const messageTransmitterProgram = new anchor.Program(messageTransmitterIdl as MessageTransmitter, provider);
    const tokenMessengerMinterProgram = new anchor.Program(tokenMessengerMinterIdl as TokenMessengerMinter, provider);
    return { messageTransmitterProgram, tokenMessengerMinterProgram };
  }

export const getDepositForBurnPdas = (
    {messageTransmitterProgram, tokenMessengerMinterProgram}: ReturnType<typeof getPrograms>,
    usdcAddress: PublicKey,
    destinationDomain: Number,
) => {
    const messageTransmitterAccount = findProgramAddress("message_transmitter", messageTransmitterProgram.programId);
    const tokenMessengerAccount = findProgramAddress("token_messenger", tokenMessengerMinterProgram.programId);
    const tokenMinterAccount = findProgramAddress("token_minter", tokenMessengerMinterProgram.programId);
    const localToken = findProgramAddress("local_token", tokenMessengerMinterProgram.programId, [usdcAddress]);
    const remoteTokenMessengerKey = findProgramAddress("remote_token_messenger", tokenMessengerMinterProgram.programId, [
        destinationDomain.toString(),
    ]);
    const authorityPda = findProgramAddress("sender_authority", tokenMessengerMinterProgram.programId);

    return {
        messageTransmitterAccount,
        tokenMessengerAccount,
        tokenMinterAccount,
        localToken,
        remoteTokenMessengerKey,
        authorityPda
    }
}

export const getReceiveMessagePdas = async (
    {messageTransmitterProgram, tokenMessengerMinterProgram}: ReturnType<typeof getPrograms>,
    solUsdcAddress: PublicKey,
    remoteUsdcAddressHex: string,
    remoteDomain: string,
    nonce: string
) => {
    const tokenMessengerAccount = findProgramAddress("token_messenger", tokenMessengerMinterProgram.programId);
    const messageTransmitterAccount = findProgramAddress("message_transmitter", messageTransmitterProgram.programId);
    const tokenMinterAccount = findProgramAddress("token_minter", tokenMessengerMinterProgram.programId);
    const localToken = findProgramAddress("local_token", tokenMessengerMinterProgram.programId, [solUsdcAddress]);
    const remoteTokenMessengerKey = findProgramAddress("remote_token_messenger", tokenMessengerMinterProgram.programId, [remoteDomain]);
    const remoteTokenKey = new PublicKey(hexToBytes(remoteUsdcAddressHex));
    const tokenPair = findProgramAddress("token_pair", tokenMessengerMinterProgram.programId, [
        remoteDomain,
        remoteTokenKey,
    ]);
    const custodyTokenAccount = findProgramAddress("custody", tokenMessengerMinterProgram.programId, [
        solUsdcAddress,
    ]);
    const authorityPda = findProgramAddress(
        "message_transmitter_authority",
        messageTransmitterProgram.programId,
        [tokenMessengerMinterProgram.programId]
    ).publicKey;
    const tokenMessengerEventAuthority = findProgramAddress("__event_authority", tokenMessengerMinterProgram.programId);

    const usedNonces = await messageTransmitterProgram.methods
        .getNoncePda({
            nonce: new anchor.BN(nonce),
            sourceDomain: Number(remoteDomain)
        })
        .accounts({
            messageTransmitter: messageTransmitterAccount.publicKey
        })
        .view();

    return {
        messageTransmitterAccount,
        tokenMessengerAccount,
        tokenMinterAccount,
        localToken,
        remoteTokenMessengerKey,
        remoteTokenKey,
        tokenPair,
        custodyTokenAccount,
        authorityPda,
        tokenMessengerEventAuthority,
        usedNonces
    }
}

export const solanaAddressToHex = (solanaAddress: string): string =>
    hexlify(bs58.decode(solanaAddress));

export const evmAddressToSolana = (evmAddress: string): string =>
    bs58.encode(hexToBytes(evmAddress));

export const evmAddressToBytes32 = (address: string): string => `0x000000000000000000000000${address.replace("0x", "")}`;

export const hexToBytes = (hex: string): Buffer => Buffer.from(hex.replace("0x", ""), "hex");

// Convenience wrapper for PublicKey.findProgramAddressSync
export const findProgramAddress = (
    label: string,
    programId: PublicKey,
    extraSeeds: (string | number[] | Buffer | PublicKey)[] = []
): FindProgramAddressResponse => {
    const seeds: Buffer[] = [Buffer.from(encode(label))];
    for (const extraSeed of extraSeeds) {
        if (typeof extraSeed === "string") {
            seeds.push(Buffer.from(encode(extraSeed)));
        } else if (Array.isArray(extraSeed)) {
            // Convert number[] to Buffer via Uint8Array
            seeds.push(Buffer.from(new Uint8Array(extraSeed)));
        } else if (Buffer.isBuffer(extraSeed)) {
            seeds.push(extraSeed);
        } else if (extraSeed instanceof PublicKey) {
            seeds.push(extraSeed.toBuffer());
        } else {
            throw new Error("Unsupported seed type in findProgramAddress");
        }
    }
    const res = PublicKey.findProgramAddressSync(seeds, programId);
    return { publicKey: res[0], bump: res[1] };
};

// Fetches attestation from attestation service given the txHash
export const getAttestation = async (txHash: string, domainId: number, maxRetries: number, retryDelay: number) => {
    let attestationResponse: any = {};
    let attempts = 0;
    while (
        (attestationResponse.error ||
        !attestationResponse.messages ||
        attestationResponse.messages?.[0]?.attestation === 'PENDING') &&
        attempts < maxRetries
    ) {
        console.log(`[Attempt: ${attempts}] Fetching attestation for ${txHash} ...`);

        const response = await fetch(`${IRIS_API_URL}/messages/${domainId}/${txHash}`);
        attestationResponse = await response.json();

        // Wait retryDelay seconds to avoid getting rate limited
        if (attestationResponse.error || !attestationResponse.messages || attestationResponse.messages?.[0]?.attestation === 'PENDING') {
            await new Promise(r => setTimeout(r, retryDelay));
        } else {
            break;
        }
        attempts++;

    }
    if (attempts === maxRetries) {
        throw new Error("Max retries reached while fetching attestation messages.");
    }
    const { message, attestation } = attestationResponse.messages[0];

    return { message, attestation };
}

export const decodeEventNonceFromMessage = (messageHex: string): string => {
    const nonceIndex = 12;
    const nonceBytesLength = 8;
    const message = hexToBytes(messageHex);
    const eventNonceBytes = message.subarray(nonceIndex, nonceIndex + nonceBytesLength);
    const eventNonceHex = hexlify(eventNonceBytes);
    return BigInt(eventNonceHex).toString();
};