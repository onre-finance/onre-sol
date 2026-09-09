import type { Connection, PublicKey, Transaction, TransactionSignature } from "@solana/web3.js";

export type PrimitiveIdlType = "bool" | "i64" | "pubkey" | "string" | "u8" | "u16" | "u32" | "u64";
export type IdlType = PrimitiveIdlType | { option: IdlType } | { vec: IdlType } | { array: [IdlType, number] } | { defined: { name: string } };

export interface Idl {
    address: string;
    instructions: IdlInstruction[];
    types?: IdlTypeDef[];
}

export interface IdlInstruction {
    name: string;
    docs?: string[];
    discriminator: number[];
    accounts?: IdlAccount[];
    args?: IdlArg[];
    returns?: IdlType;
}

export interface IdlArg {
    name: string;
    type: IdlType;
}

export interface IdlAccount {
    name: string;
    docs?: string[];
    signer?: boolean;
    writable?: boolean;
    address?: string;
    pda?: IdlPda;
    accounts?: IdlAccount[];
}

export interface IdlPda {
    seeds: IdlSeed[];
    program?: IdlSeed;
}

export type IdlSeed = { kind: "const"; value: number[] } | { kind: "account"; path: string; account?: string } | { kind: "arg"; path: string };

export interface IdlTypeDef {
    name: string;
    type: { kind: "struct"; fields: IdlArg[] } | { kind: "enum"; variants: Array<{ name: string; fields?: IdlArg[] | IdlType[] }> };
}

export interface FlatAccount {
    account: IdlAccount;
    fullName: string;
    group?: string;
}

export interface SolanaWallet {
    isPhantom?: boolean;
    isSolflare?: boolean;
    publicKey?: PublicKey;
    connect: () => Promise<{ publicKey: PublicKey } | void>;
    disconnect?: () => Promise<void>;
    signTransaction?: (transaction: Transaction) => Promise<Transaction>;
}

export interface StateAccountInfo {
    boss: PublicKey;
    proposedBoss: PublicKey;
    onycMint: PublicKey;
    worker: PublicKey;
    mainOffer: PublicKey;
}

export interface OfferAccountInfo {
    tokenInMint: PublicKey;
    tokenOutMint: PublicKey;
}

export interface RedemptionOfferAccountInfo {
    offer: PublicKey;
    tokenInMint: PublicKey;
    tokenOutMint: PublicKey;
}

export interface RedemptionRequestAccountInfo {
    offer: PublicKey;
    requestId: string;
    redeemer: PublicKey;
}

export interface ConfigurableVaultAccountInfo {
    kind: number;
    withdrawalDestination: PublicKey;
}

export type DecodedAccount =
    | { kind: "offer"; value: OfferAccountInfo }
    | { kind: "redemption_offer"; value: RedemptionOfferAccountInfo }
    | { kind: "redemption_request"; value: RedemptionRequestAccountInfo }
    | { kind: "configurable_vault"; value: ConfigurableVaultAccountInfo };

export interface AppState {
    rpcUrl: string;
    customRpcUrl?: string;
    programId: PublicKey;
    connection: Connection;
    selectedInstructionName: string;
    search: string;
    accountValues: Record<string, string>;
    accountAuto: Record<string, boolean>;
    argValues: Record<string, string>;
    derivationValues: Record<string, string>;
    instructionListScrollTop: number;
    wallet?: SolanaWallet;
    walletPublicKey?: PublicKey;
    stateInfo?: StateAccountInfo;
    decodedAccounts: Record<string, DecodedAccount>;
    accountExistence: Record<string, "exists" | "missing">;
    output: string;
    lastTransaction?: Transaction;
    lastSignature?: TransactionSignature;
}
