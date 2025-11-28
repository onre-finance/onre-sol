use trident_fuzz::fuzzing::*;

/// Storage for all account addresses used in fuzz testing.
///
/// This struct serves as a centralized repository for account addresses,
/// enabling their reuse across different instruction flows and test scenarios.
///
/// Docs: https://ackee.xyz/trident/docs/latest/trident-api-macro/trident-types/fuzz-accounts/
#[derive(Default)]
pub struct AccountAddresses {
    pub random_token_authority: AddressStorage,

    pub payer: AddressStorage,

    pub state: AddressStorage,

    pub boss: AddressStorage,

    pub onyc_mint: AddressStorage,

    pub token_mint_a: AddressStorage,

    pub token_mint_b: AddressStorage,

    pub token_mint_2022_a: AddressStorage,

    pub token_mint_2022_b: AddressStorage,

    pub offer_mint_authority: AddressStorage,

    pub offer_vault_authority: AddressStorage,

    pub permissionless_authority: AddressStorage,

    pub offer: AddressStorage,

    pub user: AddressStorage,

    pub admins: AddressStorage,
}
