import { PublicKey } from "@solana/web3.js";

/**
 * Supported network environments
 */
export type NetworkName = "mainnet-prod" | "mainnet-test" | "mainnet-dev" | "devnet-test" | "devnet-dev";

/**
 * Complete configuration for a network environment
 */
export interface NetworkConfig {
    /** Network identifier */
    name: NetworkName;

    /** Solana RPC endpoint URL */
    rpcUrl: string;

    /** Program ID for the onreapp program */
    programId: PublicKey;

    /** Squad multisig address that controls the program */
    boss: PublicKey;

    /** Token mint addresses */
    mints: {
        usdc: PublicKey;
        onyc: PublicKey;
        usdg: PublicKey;
    };
}

// RPC URLs
const MAINNET_RPC_URL = process.env.SOL_MAINNET_RPC_URL || "https://api.mainnet-beta.solana.com";
const DEVNET_RPC_URL = process.env.SOL_DEVNET_RPC_URL || "https://api.devnet.solana.com";

// Squad addresses
const PROD_SQUAD = new PublicKey("45YnzauhsBM8CpUz96Djf8UG5vqq2Dua62wuW9H3jaJ5");
const DEV_SQUAD = new PublicKey("7rzEKejyAXJXMkGfRhMV9Vg1k7tFznBBEFu3sfLNz8LC");
const DEVNET_SQUAD = new PublicKey("EVdiVScB7LX1P3bn7ZLmLJTBrSSgRXPqRU3bVxrEpRb5");

// Program IDs
const PROD_PROGRAM_ID = new PublicKey("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe");
const TEST_PROGRAM_ID = new PublicKey("J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2");
const DEV_PROGRAM_ID = new PublicKey("devHfQHgiFNifkLW49RCXpyTUZMyKuBNnFSbrQ8XsbX");

// Token mints - mainnet
const USDC_MINT = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const ONYC_MINT = new PublicKey("5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5");
const USDG_MINT = new PublicKey("2u1tszSeqZ3qBWF3uNGPFc8TzMk2tdiwknnRMWGWjGWH");

const USDC_TEST_MAINNET = new PublicKey("6ioCVbecopp6AfrPvzgZdtoaftMq4hXouDun2ekkkVvt");
const ONYC_TEST_MAINNET = new PublicKey("5Uzafw84V9rCTmYULqdJA115K6zHP16vR15zrcqa6r6C");
const USDG_TEST_MAINNET = new PublicKey("Fuisp2hZfWdqZJoRjbfoTR47DnvB8gVJFJp2ANstzbDc");

// Token mints - Devnet
const MOCK_USDC_DEVNET = new PublicKey("2eW3HJzbgrCnV1fd7dUbyPj5T95D35oBPcJyfXtoGNrw");
const MOCK_ONYC_DEVNET = new PublicKey("6WLYBF2o3RSkZ9SoNhhFYxUPYzLaa83xSTZ3o46cg4CN");
const MOCK_USDG_DEVNET = new PublicKey("HyVoVvMHRr6p1FfGSWrWDPk6bn4FAmCjajzv6SY3DHk");

const DEV_ONYC_DEVNET = new PublicKey("HQmHPQLhuXTj8dbsLUoFsJeCZWBkK75Zwczxork8Byzh");

/**
 * All network configurations
 */
export const NETWORK_CONFIGS: Record<NetworkName, NetworkConfig> = {
    "mainnet-prod": {
        name: "mainnet-prod",
        rpcUrl: MAINNET_RPC_URL,
        programId: PROD_PROGRAM_ID,
        boss: PROD_SQUAD,
        mints: {
            usdc: USDC_MINT,
            onyc: ONYC_MINT,
            usdg: USDG_MINT
        }
    },

    "mainnet-test": {
        name: "mainnet-test",
        rpcUrl: MAINNET_RPC_URL,
        programId: TEST_PROGRAM_ID,
        boss: DEV_SQUAD,
        mints: {
            usdc: USDC_TEST_MAINNET,
            onyc: ONYC_TEST_MAINNET,
            usdg: USDG_TEST_MAINNET
        }
    },

    "mainnet-dev": {
        name: "mainnet-dev",
        rpcUrl: MAINNET_RPC_URL,
        programId: DEV_PROGRAM_ID,
        boss: DEV_SQUAD,
        mints: {
            usdc: USDC_TEST_MAINNET,
            onyc: ONYC_TEST_MAINNET,
            usdg: USDG_TEST_MAINNET
        }
    },

    "devnet-test": {
        name: "devnet-test",
        rpcUrl: DEVNET_RPC_URL,
        programId: TEST_PROGRAM_ID,
        boss: DEVNET_SQUAD,
        mints: {
            usdc: MOCK_USDC_DEVNET,
            onyc: MOCK_ONYC_DEVNET,
            usdg: MOCK_USDG_DEVNET
        }
    },

    "devnet-dev": {
        name: "devnet-dev",
        rpcUrl: DEVNET_RPC_URL,
        programId: DEV_PROGRAM_ID,
        boss: DEVNET_SQUAD,
        mints: {
            usdc: MOCK_USDC_DEVNET,
            onyc: DEV_ONYC_DEVNET,
            usdg: MOCK_USDG_DEVNET
        }
    }
};

/**
 * Get the active network configuration based on NETWORK environment variable.
 * Defaults to "mainnet-prod" if not set.
 *
 * @throws Error if NETWORK is set to an invalid value
 */
export function getNetworkConfig(): NetworkConfig {
    const networkEnv = process.env.NETWORK as NetworkName | undefined;

    // Default to mainnet-prod for backward compatibility
    if (!networkEnv) {
        return NETWORK_CONFIGS["mainnet-prod"];
    }

    const config = NETWORK_CONFIGS[networkEnv];
    if (!config) {
        const validNetworks = Object.keys(NETWORK_CONFIGS).join(", ");
        throw new Error(
            `Invalid NETWORK environment variable: "${networkEnv}". ` +
            `Valid values are: ${validNetworks}`
        );
    }

    return config;
}

/**
 * Print current configuration summary to console.
 * Useful for verifying correct environment before executing operations.
 */
export function printConfigSummary(config: NetworkConfig): void {
    console.log("═══════════════════════════════════════════════════════════════");
    console.log(`  NETWORK: ${config.name.toUpperCase()}`);
    console.log("═══════════════════════════════════════════════════════════════");
    console.log(`  RPC:     ${config.rpcUrl}`);
    console.log(`  Program: ${config.programId.toBase58()}`);
    console.log(`  Boss:    ${config.boss.toBase58()}`);
    console.log(`  USDC:    ${config.mints.usdc.toBase58()}`);
    console.log(`  ONyc:    ${config.mints.onyc.toBase58()}`);
    console.log(`  USDG:    ${config.mints.usdg.toBase58()}`);
    console.log("═══════════════════════════════════════════════════════════════\n");
}
