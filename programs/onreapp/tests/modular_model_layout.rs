use anchor_lang::{AccountDeserialize, AccountSerialize, AnchorSerialize, Discriminator, Space};
use onreapp::{instructions, model, state};

/// Pin the proposed Borsh wire sizes, including a complete serialize/deserialize pass.
fn check_layout<T: AccountDeserialize + AccountSerialize + Discriminator + Space>(
    body_size: usize,
) {
    assert_eq!(T::DISCRIMINATOR.len(), 8);
    assert_eq!(T::INIT_SPACE, body_size);
    let mut bytes = T::DISCRIMINATOR.to_vec();
    bytes.resize(8 + body_size, 0);
    let account = T::try_deserialize(&mut bytes.as_slice()).unwrap();
    let mut encoded = Vec::new();
    account.try_serialize(&mut encoded).unwrap();
    assert_eq!(encoded, bytes);
}

#[test]
fn model_account_wire_sizes_are_fixed() {
    check_layout::<model::ManagedTokenConfig>(51);
    check_layout::<model::Pricer>(444);
    check_layout::<model::Quoter>(11);
    check_layout::<model::PropAmmState>(183);
    check_layout::<model::FeeConfig>(52);
    check_layout::<model::VaultConfig>(44);
    check_layout::<model::ManagedTokenBufferState>(166);
    check_layout::<model::OfferConfig>(230);
    check_layout::<model::FulfillmentRequest>(89);
    check_layout::<model::ManagedTokenSupplyExclusions>(674);
}

#[test]
fn model_accounts_cannot_be_mistaken_for_legacy_accounts() {
    let legacy = [
        state::State::DISCRIMINATOR,
        state::PermissionlessAuthority::DISCRIMINATOR,
        state::MarketStats::DISCRIMINATOR,
        state::CirculatingSupplyExcludedAccounts::DISCRIMINATOR,
        state::CirculatingSupplyExcludedBalance::DISCRIMINATOR,
        state::ConfigurableVault::DISCRIMINATOR,
        instructions::Offer::DISCRIMINATOR,
        instructions::RedemptionOffer::DISCRIMINATOR,
        instructions::RedemptionRequest::DISCRIMINATOR,
        instructions::PropAmmPairState::DISCRIMINATOR,
        instructions::BufferState::DISCRIMINATOR,
    ];
    let target = [
        model::ManagedTokenConfig::DISCRIMINATOR,
        model::Pricer::DISCRIMINATOR,
        model::Quoter::DISCRIMINATOR,
        model::PropAmmState::DISCRIMINATOR,
        model::FeeConfig::DISCRIMINATOR,
        model::VaultConfig::DISCRIMINATOR,
        model::ManagedTokenBufferState::DISCRIMINATOR,
        model::OfferConfig::DISCRIMINATOR,
        model::FulfillmentRequest::DISCRIMINATOR,
        model::ManagedTokenSupplyExclusions::DISCRIMINATOR,
    ];
    for (index, discriminator) in target.iter().enumerate() {
        assert!(!legacy.contains(discriminator));
        assert!(!target[..index].contains(discriminator));
    }
}

fn encode(value: impl AnchorSerialize) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.serialize(&mut bytes).unwrap();
    bytes
}

#[test]
fn enum_ordinals_match_evm_and_remain_stable_for_pda_seeds() {
    let cases = [
        (encode(model::PricingDenomination::Usd), 0),
        (encode(model::OfferFlow::Permissioned), 0),
        (encode(model::OfferFlow::Permissionless), 1),
        (encode(model::OfferFlow::Worker), 2),
        (encode(model::OfferDirection::AssetToManaged), 0),
        (encode(model::OfferDirection::ManagedToAsset), 1),
        (encode(model::QuoterKind::Nav), 0),
        (encode(model::QuoterKind::NavPermissionless), 1),
        (encode(model::QuoterKind::PropAmm), 2),
        (encode(model::VaultKind::Fee), 0),
        (encode(model::VaultKind::Proceeds), 1),
        (encode(model::VaultKind::Liquidity), 2),
        (encode(model::VaultKind::BufferReserve), 3),
    ];
    for (encoded, ordinal) in cases {
        assert_eq!(encoded, vec![ordinal]);
    }
}

#[test]
fn configuration_seed_layouts_are_disjoint() {
    use model::seeds as new;
    use onreapp::constants::seeds as legacy;

    // PDA hashing concatenates seed bytes without delimiters. Model every dynamic
    // byte as a wildcard, including enum tags: this checks all possible values,
    // rather than sampling pubkeys or assuming different seed boundaries suffice.
    fn layout(prefix: &[u8], dynamic_bytes: usize) -> Vec<Option<u8>> {
        prefix
            .iter()
            .copied()
            .map(Some)
            .chain(std::iter::repeat_n(None, dynamic_bytes))
            .collect()
    }

    fn can_alias(a: &[Option<u8>], b: &[Option<u8>]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(a, b)| a.is_none() || b.is_none() || a == b)
    }

    let current = [
        layout(new::MANAGED_TOKEN, 32),
        layout(new::PRICER, 32 + 1 + 8),
        layout(new::QUOTER, 1 + 8),
        layout(new::PROP_AMM, 32),
        layout(new::FEE_CONFIG, 8),
        layout(new::VAULT, 1 + 8),
        layout(new::OFFER_CONFIG, 32 + 32 + 1),
        layout(new::BUFFER, 32),
        layout(new::BUFFER_RESERVE, 32),
        layout(new::BUFFER_MANAGEMENT, 32),
        layout(new::BUFFER_PERFORMANCE, 32),
    ];
    let mut previous: Vec<_> = [
        legacy::STATE,
        legacy::OFFER_VAULT_AUTHORITY,
        legacy::PERMISSIONLESS_AUTHORITY,
        legacy::MINT_AUTHORITY,
        legacy::MARKET_STATS,
        legacy::CIRCULATING_SUPPLY_EXCLUDED_ACCOUNTS,
        legacy::CIRCULATING_SUPPLY_EXCLUDED_BALANCE,
        legacy::REDEMPTION_OFFER_VAULT_AUTHORITY,
        legacy::NONCE_ACCOUNT,
        legacy::BUFFER_STATE,
        legacy::RESERVE_VAULT_AUTHORITY,
    ]
    .into_iter()
    .map(|seed| layout(seed, 0))
    .collect();
    previous.extend([
        layout(legacy::OFFER, 32 + 32),
        layout(legacy::REDEMPTION_OFFER, 32 + 32),
        layout(legacy::REDEMPTION_REQUEST, 32 + 32 + 32),
        layout(legacy::PROP_AMM_PAIR_STATE, 32),
    ]);
    use state::ConfigurableVaultKind::*;
    for kind in [
        OfferFee,
        ManagementFee,
        PerformanceFee,
        PropAmmBuyFee,
        OfferProceeds,
        PropAmmProceeds,
        PermissionlessOfferFee,
        RedemptionFee,
        PropAmmSellFee,
    ] {
        previous.push(layout(
            &[legacy::CONFIGURABLE_VAULT, kind.seed()].concat(),
            0,
        ));
    }

    for (index, candidate) in current.iter().enumerate() {
        for other in &current[..index] {
            assert!(!can_alias(candidate, other), "new seed layouts overlap");
        }
        for old in &previous {
            assert!(!can_alias(candidate, old), "new and legacy layouts overlap");
        }
    }
}
