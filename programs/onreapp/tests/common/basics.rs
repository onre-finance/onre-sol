use super::*;

pub const PROGRAM_ID: Pubkey = solana_sdk::pubkey!("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe");
pub const TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const ATA_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");
pub const BPF_UPGRADEABLE_LOADER_ID: Pubkey =
    solana_sdk::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");
pub const ED25519_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("Ed25519SigVerify111111111111111111111111111");
pub const SYSVAR_INSTRUCTIONS_ID: Pubkey =
    solana_sdk::pubkey!("Sysvar1nstructions1111111111111111111111111");

pub const INITIAL_LAMPORTS: u64 = 1_000_000_000;
pub const MAX_ADMINS: usize = 20;

pub const STATE_SEED: &[u8] = b"state";
pub const OFFER_SEED: &[u8] = b"offer";
pub const OFFER_VAULT_AUTHORITY_SEED: &[u8] = b"offer_vault_authority";
pub const REDEMPTION_OFFER_VAULT_AUTHORITY_SEED: &[u8] = b"redemption_offer_vault_authority";
pub const REDEMPTION_FEE_VAULT_AUTHORITY_SEED: &[u8] = b"redemption_fee_vault_authority";
pub const PERMISSIONLESS_AUTHORITY_SEED: &[u8] = b"permissionless-1";
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";
pub const MARKET_STATS_SEED: &[u8] = b"market_stats";
pub const BUFFER_STATE_SEED: &[u8] = b"buffer_state";
pub const RESERVE_VAULT_AUTHORITY_SEED: &[u8] = b"reserve_vault_authority";
pub const MANAGEMENT_FEE_VAULT_AUTHORITY_SEED: &[u8] = b"management_fee_vault_authority";
pub const PERFORMANCE_FEE_VAULT_AUTHORITY_SEED: &[u8] = b"performance_fee_vault_authority";
pub const REDEMPTION_OFFER_SEED: &[u8] = b"redemption_offer";
pub const REDEMPTION_REQUEST_SEED: &[u8] = b"redemption_request";

pub fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
        &ATA_PROGRAM_ID,
    )
    .0
}

pub fn get_associated_token_address_2022(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            wallet.as_ref(),
            TOKEN_2022_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0
}

pub fn derive_ata(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ATA_PROGRAM_ID,
    )
    .0
}

pub fn anchor_discriminator(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{}:{}", namespace, name);
    let hash = solana_sdk::hash::hash(preimage.as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash.to_bytes()[..8]);
    disc
}

pub fn ix_discriminator(name: &str) -> [u8; 8] {
    anchor_discriminator("global", name)
}

pub fn find_state_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[STATE_SEED], &PROGRAM_ID)
}

pub fn find_offer_pda(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[OFFER_SEED, token_in_mint.as_ref(), token_out_mint.as_ref()],
        &PROGRAM_ID,
    )
}

pub fn find_offer_vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[OFFER_VAULT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_redemption_vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[REDEMPTION_OFFER_VAULT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_redemption_fee_vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[REDEMPTION_FEE_VAULT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_permissionless_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PERMISSIONLESS_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_mint_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_market_stats_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MARKET_STATS_SEED], &PROGRAM_ID)
}

pub fn find_buffer_state_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[BUFFER_STATE_SEED], &PROGRAM_ID)
}

pub fn find_reserve_vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[RESERVE_VAULT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_management_fee_vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MANAGEMENT_FEE_VAULT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_performance_fee_vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PERFORMANCE_FEE_VAULT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn find_program_data_pda() -> Pubkey {
    Pubkey::find_program_address(&[PROGRAM_ID.as_ref()], &BPF_UPGRADEABLE_LOADER_ID).0
}

pub fn find_redemption_offer_pda(token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            REDEMPTION_OFFER_SEED,
            token_in_mint.as_ref(),
            token_out_mint.as_ref(),
        ],
        &PROGRAM_ID,
    )
}

pub fn find_redemption_request_pda(redemption_offer: &Pubkey, counter: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            REDEMPTION_REQUEST_SEED,
            redemption_offer.as_ref(),
            &counter.to_le_bytes(),
        ],
        &PROGRAM_ID,
    )
}
