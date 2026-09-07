import { Buffer } from "buffer";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import type { ConfigurableVaultAccountInfo, DecodedAccount, OfferAccountInfo, RedemptionOfferAccountInfo, RedemptionRequestAccountInfo, StateAccountInfo } from "./types";

export const STATE_ACCOUNT_MIN_LENGTH = 890;
export const STATE_ACCOUNT_OFFSETS = {
    boss: 8,
    proposedBoss: 40,
    onycMint: 73,
    worker: 818,
    mainOffer: 858,
} as const;

export function decodeKnownAccount(kind: DecodedAccount["kind"], data: Buffer | Uint8Array): DecodedAccount | undefined {
    const bytes = Buffer.from(data);
    if (kind === "offer") {
        const value = decodeOfferAccount(bytes);
        return value ? { kind, value } : undefined;
    }
    if (kind === "redemption_offer") {
        const value = decodeRedemptionOfferAccount(bytes);
        return value ? { kind, value } : undefined;
    }
    if (kind === "redemption_request") {
        const value = decodeRedemptionRequestAccount(bytes);
        return value ? { kind, value } : undefined;
    }
    const value = decodeConfigurableVaultAccount(bytes);
    return value ? { kind, value } : undefined;
}

export function decodeStateAccount(data: Buffer | Uint8Array): StateAccountInfo | undefined {
    const bytes = Buffer.from(data);
    if (bytes.length < STATE_ACCOUNT_MIN_LENGTH) return undefined;
    return {
        boss: publicKeyAt(bytes, STATE_ACCOUNT_OFFSETS.boss),
        proposedBoss: publicKeyAt(bytes, STATE_ACCOUNT_OFFSETS.proposedBoss),
        onycMint: publicKeyAt(bytes, STATE_ACCOUNT_OFFSETS.onycMint),
        worker: publicKeyAt(bytes, STATE_ACCOUNT_OFFSETS.worker),
        mainOffer: publicKeyAt(bytes, STATE_ACCOUNT_OFFSETS.mainOffer),
    };
}

function decodeOfferAccount(bytes: Buffer): OfferAccountInfo | undefined {
    if (bytes.length < 72) return undefined;
    return {
        tokenInMint: publicKeyAt(bytes, 8),
        tokenOutMint: publicKeyAt(bytes, 40),
    };
}

function decodeRedemptionOfferAccount(bytes: Buffer): RedemptionOfferAccountInfo | undefined {
    if (bytes.length < 138) return undefined;
    return {
        offer: publicKeyAt(bytes, 8),
        tokenInMint: publicKeyAt(bytes, 40),
        tokenOutMint: publicKeyAt(bytes, 72),
    };
}

function decodeRedemptionRequestAccount(bytes: Buffer): RedemptionRequestAccountInfo | undefined {
    if (bytes.length < 76) return undefined;
    const requestIdLength = bytes.readUInt32LE(40);
    const requestIdEnd = 44 + requestIdLength;
    if (requestIdLength !== 32 || bytes.length < requestIdEnd + 32) return undefined;
    return {
        offer: publicKeyAt(bytes, 8),
        requestId: bytes.subarray(44, requestIdEnd).toString("utf8"),
        redeemer: publicKeyAt(bytes, requestIdEnd),
    };
}

function decodeConfigurableVaultAccount(bytes: Buffer): ConfigurableVaultAccountInfo | undefined {
    if (bytes.length < 41) return undefined;
    return {
        kind: bytes[8],
        withdrawalDestination: publicKeyAt(bytes, 9),
    };
}

export function sameStateInfo(left: StateAccountInfo | undefined, right: StateAccountInfo): boolean {
    return Boolean(
        left &&
        left.boss.equals(right.boss) &&
        left.proposedBoss.equals(right.proposedBoss) &&
        left.onycMint.equals(right.onycMint) &&
        left.worker.equals(right.worker) &&
        left.mainOffer.equals(right.mainOffer),
    );
}

export function publicKeyAt(bytes: Buffer, offset: number): PublicKey {
    return new PublicKey(bytes.subarray(offset, offset + 32));
}

export function isDefaultPublicKey(publicKey: PublicKey): boolean {
    return publicKey.equals(SystemProgram.programId);
}

export function u64Seed(value: bigint): Buffer {
    const buffer = Buffer.alloc(8);
    buffer.writeBigUInt64LE(value);
    return buffer;
}
