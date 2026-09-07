use anchor_lang::{AccountDeserialize, AnchorSerialize, Discriminator, Space};
use onreapp::constants::MAX_ADMINS;
use onreapp::instructions::{RedemptionOffer, RedemptionRequest};
use onreapp::state::State;
use solana_sdk::pubkey::Pubkey;

#[derive(AnchorSerialize)]
struct LegacyState {
    boss: Pubkey,
    proposed_boss: Pubkey,
    is_killed: bool,
    onyc_mint: Pubkey,
    admins: [Pubkey; MAX_ADMINS],
    approver1: Pubkey,
    approver2: Pubkey,
    bump: u8,
    max_supply: u64,
    // Legacy field name; V5 renames this same serialized Pubkey slot to `worker`.
    redemption_admin: Pubkey,
    reserved: [u8; 96],
}

#[derive(AnchorSerialize)]
struct LegacyRedemptionOffer {
    offer: Pubkey,
    token_in_mint: Pubkey,
    token_out_mint: Pubkey,
    executed_redemptions: u128,
    requested_redemptions: u128,
    fee_basis_points: u16,
    gap: u64,
    bump: u8,
    reserved: [u8; 109],
}

fn account_bytes<T: AnchorSerialize>(discriminator: &[u8], account: &T) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(discriminator);
    account.serialize(&mut data).unwrap();
    data
}

#[test]
fn redemption_request_preserves_allocation_size_with_uuid() {
    let request = RedemptionRequest {
        offer: Pubkey::new_unique(),
        request_id: "12345678901234567890123456789012".to_string(),
        redeemer: Pubkey::new_unique(),
        amount: 500_000,
        bump: 249,
        fulfilled_amount: 100_000,
        reserved: [0; 91],
    };
    let data = account_bytes(RedemptionRequest::DISCRIMINATOR, &request);
    assert_eq!(8 + RedemptionRequest::INIT_SPACE, 216);
    assert_eq!(data.len(), 216);
    let decoded = RedemptionRequest::try_deserialize(&mut data.as_slice()).unwrap();
    assert_eq!(decoded.request_id, request.request_id);
    assert_eq!(decoded.redeemer, request.redeemer);
    assert_eq!(decoded.fulfilled_amount, request.fulfilled_amount);
}

#[test]
fn state_deserializes_legacy_master_layout() {
    let boss = Pubkey::new_unique();
    let proposed_boss = Pubkey::new_unique();
    let onyc_mint = Pubkey::new_unique();
    let redemption_admin = Pubkey::new_unique();
    let approver1 = Pubkey::new_unique();
    let approver2 = Pubkey::new_unique();
    let mut admins = [Pubkey::default(); MAX_ADMINS];
    admins[0] = Pubkey::new_unique();

    let legacy = LegacyState {
        boss,
        proposed_boss,
        is_killed: true,
        onyc_mint,
        admins,
        approver1,
        approver2,
        bump: 254,
        max_supply: 1_000_000_000,
        redemption_admin,
        reserved: [0; 96],
    };

    let mut body = Vec::new();
    legacy.serialize(&mut body).unwrap();
    assert_eq!(body.len(), State::INIT_SPACE);

    let data = account_bytes(State::DISCRIMINATOR, &legacy);
    let mut slice = data.as_slice();
    let state = State::try_deserialize(&mut slice).unwrap();

    assert_eq!(state.boss, boss);
    assert_eq!(state.proposed_boss, proposed_boss);
    assert!(state.is_killed);
    assert_eq!(state.onyc_mint, onyc_mint);
    assert_eq!(state.admins, admins);
    assert_eq!(state.approver1, approver1);
    assert_eq!(state.approver2, approver2);
    assert_eq!(state.bump, 254);
    assert_eq!(state.max_supply, 1_000_000_000);
    assert_eq!(state.worker, redemption_admin);
    assert_eq!(state.max_mint_amount, 0);
    assert_eq!(state.main_offer, Pubkey::default());
}

#[test]
fn redemption_offer_deserializes_legacy_master_layout() {
    let offer = Pubkey::new_unique();
    let token_in_mint = Pubkey::new_unique();
    let token_out_mint = Pubkey::new_unique();

    let legacy = LegacyRedemptionOffer {
        offer,
        token_in_mint,
        token_out_mint,
        executed_redemptions: 123,
        requested_redemptions: 456,
        fee_basis_points: 17,
        gap: 42,
        bump: 251,
        reserved: [0; 109],
    };

    let mut body = Vec::new();
    legacy.serialize(&mut body).unwrap();
    assert_eq!(body.len(), RedemptionOffer::INIT_SPACE);

    let data = account_bytes(RedemptionOffer::DISCRIMINATOR, &legacy);
    let mut slice = data.as_slice();
    let redemption_offer = RedemptionOffer::try_deserialize(&mut slice).unwrap();

    assert_eq!(redemption_offer.offer, offer);
    assert_eq!(redemption_offer.token_in_mint, token_in_mint);
    assert_eq!(redemption_offer.token_out_mint, token_out_mint);
    assert_eq!(redemption_offer.executed_redemptions, 123);
    assert_eq!(redemption_offer.requested_redemptions, 456);
    assert_eq!(redemption_offer.fee_basis_points, 17);
    assert_eq!(redemption_offer.gap, 42);
    assert_eq!(redemption_offer.bump, 251);
    assert_eq!(redemption_offer.vault_target_bps, 0);
    assert!(!redemption_offer.is_disabled());
    assert_eq!(redemption_offer.fee_basis_points_prop_amm_sell, 0);
}
