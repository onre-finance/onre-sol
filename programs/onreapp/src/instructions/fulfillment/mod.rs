//! Escrowed requests against Worker-execution offers (port of EVM
//! `LibOnReFulfillment`): the user escrows the input, the worker fills it, possibly
//! partially, at the price and fee current at each fill, and the user or the worker
//! can cancel the unfilled remainder.
mod cancel_fulfillment_request;
mod create_fulfillment_request;
mod fulfill_fulfillment_request;

pub use cancel_fulfillment_request::*;
pub use create_fulfillment_request::*;
pub use fulfill_fulfillment_request::*;

use crate::model::FulfillmentRequest;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, CloseAccount, TokenAccount, TokenInterface};

/// Closes the request's empty escrow ATA, signed by the request PDA, refunding its
/// rent to the request user who paid it.
fn close_escrow<'info>(
    request: &Account<'info, FulfillmentRequest>,
    escrow: &InterfaceAccount<'info, TokenAccount>,
    user: &AccountInfo<'info>,
    token_program: &Interface<'info, TokenInterface>,
) -> Result<()> {
    request.with_signer_seeds(|seeds| {
        token_interface::close_account(CpiContext::new_with_signer(
            token_program.key(),
            CloseAccount {
                account: escrow.to_account_info(),
                destination: user.clone(),
                authority: request.to_account_info(),
            },
            &[seeds],
        ))
    })
}
