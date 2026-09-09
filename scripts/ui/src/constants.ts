import { PublicKey } from "@solana/web3.js";
import idlJson from "../../../target/idl/onreapp.json";
import type { Idl } from "./types";

export const MAINNET_PROGRAM_ID = new PublicKey("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe");
export const MAINNET_RPC_URL = "https://api.mainnet-beta.solana.com";
export const DEFAULT_RPC_PATH = "/rpc";
export const DEFAULT_PROGRAM_ID = parseProgramId(import.meta.env.VITE_ONRE_PROGRAM_ID) ?? MAINNET_PROGRAM_ID;
export const TOKEN_PROGRAM_ID = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
export const SYSVAR_INSTRUCTIONS_PUBKEY = new PublicKey("Sysvar1nstructions1111111111111111111111111");
export const PLACEHOLDER_BLOCKHASH = "11111111111111111111111111111111";

export const MAINNET_MINTS = {
    usdc: new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
    usdt: new PublicKey("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"),
    onyc: new PublicKey("5Y8NV33Vv7WbnLfq3zBcKSdYPrk7g2KoiQoe7M2tcxp5"),
    usdg: new PublicKey("2u1tszSeqZ3qBWF3uNGPFc8TzMk2tdiwknnRMWGWjGWH"),
};

export const TOKEN_CHOICES = [
    { label: "USDC", value: MAINNET_MINTS.usdc },
    { label: "USDT", value: MAINNET_MINTS.usdt },
    { label: "ONyc", value: MAINNET_MINTS.onyc },
    { label: "USDG", value: MAINNET_MINTS.usdg },
] as const;

export const CONFIGURABLE_VAULT_KIND_SEEDS: Record<string, string> = {
    OfferFee: "offer_fee",
    PermissionlessOfferFee: "permissionless_offer_fee",
    RedemptionFee: "redemption_fee",
    ManagementFee: "management_fee",
    PerformanceFee: "performance_fee",
    PropAmmBuyFee: "prop_amm_buy_fee",
    PropAmmSellFee: "prop_amm_sell_fee",
    OfferProceeds: "offer_proceeds",
    PropAmmProceeds: "prop_amm_proceeds",
};

export const CONFIGURABLE_VAULT_ACCOUNT_SEEDS: Record<string, string> = {
    offer_fee_vault: "offer_fee",
    permissionless_offer_fee_vault: "permissionless_offer_fee",
    redemption_fee_vault: "redemption_fee",
    management_fee_vault: "management_fee",
    performance_fee_vault: "performance_fee",
    prop_amm_buy_fee_vault: "prop_amm_buy_fee",
    prop_amm_sell_fee_vault: "prop_amm_sell_fee",
    offer_proceeds_vault: "offer_proceeds",
    prop_amm_proceeds_vault: "prop_amm_proceeds",
};

export const OFFER_TOKEN_IN_KEY = "offer.token_in_mint";
export const OFFER_TOKEN_OUT_KEY = "offer.token_out_mint";
export const REDEMPTION_OFFER_TOKEN_OUT_KEY = "redemption_offer.token_out_mint";
export const REDEMPTION_REQUEST_ID_KEY = "redemption_request.id";

export const PDA_SEEDS: Record<string, string> = {
    state: "state",
    offer_vault_authority: "offer_vault_authority",
    vault_authority: "offer_vault_authority",
    permissionless_authority: "permissionless-1",
    mint_authority: "mint_authority",
    buffer_state: "buffer_state",
    reserve_vault_authority: "reserve_vault_authority",
    redemption_vault_authority: "redemption_offer_vault_authority",
    market_stats: "market_stats",
    circulating_supply_excluded_balance: "circ_supply_excl_balance",
    excluded_accounts: "circ_supply_excl_accounts",
};

export const idl = { ...idlJson, address: DEFAULT_PROGRAM_ID.toBase58() } as Idl;
export const instructionByName = new Map(idl.instructions.map((ix) => [ix.name, ix]));
export const typeByName = new Map((idl.types ?? []).map((typeDef) => [typeDef.name, typeDef]));

function parseProgramId(value: string | undefined): PublicKey | undefined {
    if (!value) return undefined;
    try {
        return new PublicKey(value);
    } catch {
        return undefined;
    }
}
