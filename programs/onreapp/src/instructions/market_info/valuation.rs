//! Account-independent input for canonical market valuation.
use crate::{
    constants::MAX_VECTORS,
    instructions::Offer,
    model::{Pricer, PricingVector},
};
use anchor_lang::prelude::*;
use std::array::from_fn;

pub(crate) struct MarketValuation {
    pub managed_mint: Pubkey,
    pub vectors: [PricingVector; MAX_VECTORS],
}

impl From<&Offer> for MarketValuation {
    fn from(offer: &Offer) -> Self {
        Self {
            managed_mint: offer.token_out_mint,
            vectors: from_fn(|i| PricingVector::from(&offer.vectors[i])),
        }
    }
}

impl From<&Pricer> for MarketValuation {
    fn from(pricer: &Pricer) -> Self {
        Self {
            managed_mint: pricer.managed_mint,
            vectors: pricer.vectors,
        }
    }
}
