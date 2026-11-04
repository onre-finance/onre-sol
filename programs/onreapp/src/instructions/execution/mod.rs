//! Execution against reusable configuration, alongside the unchanged V2 adapters.

mod accounts;
mod buffer;
mod liquidity;
mod permissions;
mod quote;
mod settlement;
mod take_offer;
mod validation;

pub use accounts::*;
pub use buffer::ManagedTokenBufferAccruedEvent;
pub use take_offer::*;
