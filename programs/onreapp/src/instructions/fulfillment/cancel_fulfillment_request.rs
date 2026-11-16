//! Cancellation of a fulfillment request by its user or the worker.
use super::close_escrow;
use crate::{
    constants::seeds::STATE,
    model::{seeds::FULFILLMENT_REQUEST, FulfillmentRequest, OfferConfig},
    state::State,
    utils::transfer_tokens,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[event]
pub struct FulfillmentRequestCancelledEvent {
    pub fulfillment_request: Pubkey,
    pub offer_config: Pubkey,
    pub user: Pubkey,
    /// Escrow balance returned to the user: the unfilled remainder, plus any
    /// unsolicited surplus.
    pub returned_amount: u64,
    pub cancelled_by: Pubkey,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct CancelFulfillmentRequest<'info> {
    #[account(
        mut,
        seeds = [
            FULFILLMENT_REQUEST,
            fulfillment_request.offer_config.as_ref(),
            fulfillment_request.user.as_ref(),
            &fulfillment_request.request_id.to_le_bytes()
        ],
        bump = fulfillment_request.bump,
        has_one = offer_config @ OnreError::FulfillmentRequestMismatch,
        has_one = user @ OnreError::FulfillmentRequestMismatch,
        close = user
    )]
    pub fulfillment_request: Box<Account<'info, FulfillmentRequest>>,

    /// The request's offer, which fixes the escrowed mint. Need not be enabled.
    pub offer_config: Box<Account<'info, OfferConfig>>,

    /// CHECK: The request's user, by `has_one`: receives the escrow and the rent.
    #[account(mut)]
    pub user: UncheckedAccount<'info>,

    /// The request's user or the worker.
    #[account(
        mut,
        constraint = signer.key() == fulfillment_request.user || signer.key() == state.worker
            @ OnreError::Unauthorized
    )]
    pub signer: Signer<'info>,

    #[account(seeds = [STATE], bump = state.bump)]
    pub state: Box<Account<'info, State>>,

    #[account(
        address = offer_config.token_in_mint @ OnreError::InvalidTokenInMint,
        mint::token_program = token_in_program
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = fulfillment_request,
        associated_token::token_program = token_in_program
    )]
    pub escrow_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Recreated at the signer's expense if the user closed it.
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = token_in_mint,
        associated_token::authority = user,
        associated_token::token_program = token_in_program
    )]
    pub user_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_in_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

/// Returns the whole escrow balance (the unfilled remainder) to the user and
/// closes the request and its escrow, refunding their rent to the user. Allowed
/// while the kill switch is active, so users can always recover escrowed input.
pub fn cancel_fulfillment_request(ctx: Context<CancelFulfillmentRequest>) -> Result<()> {
    let a = ctx.accounts;
    let returned_amount = a.escrow_token_in.amount;
    if returned_amount > 0 {
        a.fulfillment_request.with_signer_seeds(|seeds| {
            transfer_tokens(
                &a.token_in_mint,
                &a.token_in_program,
                &a.escrow_token_in,
                &a.user_token_in,
                &a.fulfillment_request.to_account_info(),
                Some(&[seeds]),
                returned_amount,
            )
        })?;
    }
    close_escrow(
        &a.fulfillment_request,
        &a.escrow_token_in,
        &a.user.to_account_info(),
        &a.token_in_program,
    )?;
    emit!(FulfillmentRequestCancelledEvent {
        fulfillment_request: a.fulfillment_request.key(),
        offer_config: a.fulfillment_request.offer_config,
        user: a.user.key(),
        returned_amount,
        cancelled_by: a.signer.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
