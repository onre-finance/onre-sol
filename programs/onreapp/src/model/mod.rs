//! EVM-aligned configuration model, initialized by `instructions::configuration`.
//!
//! Existing settlement accounts remain authoritative until a separately implemented migration.
//! See `docs/MODULAR_MODEL.md` for identities, relationships, and future validation rules.

use anchor_lang::prelude::Pubkey;

pub mod buffer;
pub mod fee;
pub mod fulfillment;
pub mod managed_token;
pub mod offer;
pub mod pricer;
pub mod prop_amm;
pub mod quoter;
pub mod seeds;
pub mod supply_exclusions;
pub mod vault;

/// Optional references are stored as `Pubkey::default()` when absent, keeping
/// account layouts fixed (stable field offsets for `memcmp` filters, as with EVM's
/// `address(0)`). Program code reads them through `Option`-returning accessors built
/// on this, and writes them with `unwrap_or_default()`.
pub(crate) fn optional(key: Pubkey) -> Option<Pubkey> {
    (key != Pubkey::default()).then_some(key)
}

pub use buffer::{BufferSettings, ManagedTokenBufferState};
pub use fee::FeeConfig;
pub use fulfillment::FulfillmentRequest;
pub use managed_token::ManagedTokenConfig;
pub use offer::{OfferAccess, OfferConfig, OfferDirection, OfferExecution, OfferPermission};
pub use pricer::{Pricer, PricingDenomination, PricingVector};
pub use prop_amm::{PropAmmConfig, PropAmmState};
pub use quoter::{Quoter, QuoterKind};
pub use supply_exclusions::ManagedTokenSupplyExclusions;
pub use vault::{VaultConfig, VaultKind};
