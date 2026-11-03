//! EVM-aligned configuration model, initialized by `instructions::configuration`.
//!
//! Existing settlement accounts remain authoritative until a separately implemented migration.
//! See `docs/MODULAR_MODEL.md` for identities, relationships, and future validation rules.

pub mod buffer;
pub mod fee;
pub mod fulfillment;
pub mod managed_token;
pub mod market_stats;
pub mod offer;
pub mod pricer;
pub mod prop_amm;
pub mod quoter;
pub mod seeds;
pub mod supply_exclusions;
pub mod vault;

pub use buffer::ManagedTokenBufferState;
pub use fee::FeeConfig;
pub use fulfillment::FulfillmentRequest;
pub use managed_token::ManagedTokenConfig;
pub use market_stats::MarketStats;
pub use offer::{OfferConfig, OfferDirection, OfferFlow};
pub use pricer::{Pricer, PricingDenomination, PricingVector};
pub use prop_amm::{PropAmmConfig, PropAmmState};
pub use quoter::{Quoter, QuoterKind};
pub use supply_exclusions::ManagedTokenSupplyExclusions;
pub use vault::{VaultConfig, VaultKind};
