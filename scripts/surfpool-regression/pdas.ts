import { PublicKey } from "@solana/web3.js";

import { PROGRAM_ID } from "./constants";

const seed = (value: string) => Buffer.from(value);

export function pda(...seeds: Buffer[]): PublicKey {
    return PublicKey.findProgramAddressSync(seeds, PROGRAM_ID)[0];
}

export const PDAS = {
    state: pda(seed("state")),
    offerVaultAuthority: pda(seed("offer_vault_authority")),
    permissionlessAuthority: pda(seed("permissionless-1")),
    redemptionVaultAuthority: pda(seed("redemption_offer_vault_authority")),
    mintAuthority: pda(seed("mint_authority")),
    bufferState: pda(seed("buffer_state")),
    reserveVaultAuthority: pda(seed("reserve_vault_authority")),
    marketStats: pda(seed("market_stats")),
    circulatingSupplyExcludedBalance: pda(seed("circ_supply_excl_balance")),
};

export function offerPda(assetMint: PublicKey, onycMint: PublicKey): PublicKey {
    return pda(seed("offer"), assetMint.toBuffer(), onycMint.toBuffer());
}

export function redemptionOfferPda(onycMint: PublicKey, assetMint: PublicKey): PublicKey {
    return pda(seed("redemption_offer"), onycMint.toBuffer(), assetMint.toBuffer());
}

export function redemptionRequestPda(redemptionOffer: PublicKey, redeemer: PublicKey, requestId: string): PublicKey {
    return pda(seed("redemption_request"), redemptionOffer.toBuffer(), redeemer.toBuffer(), Buffer.from(requestId, "utf8"));
}

export function propAmmPairPda(offer: PublicKey): PublicKey {
    return pda(seed("prop_amm_pair"), offer.toBuffer());
}

export const configurableVaultSeeds = {
    offerFee: "offer_fee",
    managementFee: "management_fee",
    performanceFee: "performance_fee",
    propAmmFee: "prop_amm_fee",
    offerProceeds: "offer_proceeds",
    propAmmProceeds: "prop_amm_proceeds",
} as const;

export type ConfigurableVaultName = keyof typeof configurableVaultSeeds;

export const configurableVaultKinds: Record<ConfigurableVaultName, Record<string, object>> = {
    offerFee: { offerFee: {} },
    managementFee: { managementFee: {} },
    performanceFee: { performanceFee: {} },
    propAmmFee: { propAmmFee: {} },
    offerProceeds: { offerProceeds: {} },
    propAmmProceeds: { propAmmProceeds: {} },
};

export function configurableVaultPda(kind: ConfigurableVaultName): PublicKey {
    return pda(seed("configurable_vault"), seed(configurableVaultSeeds[kind]));
}
