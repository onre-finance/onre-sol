//! Worker fill of an escrowed fulfillment request.
use super::close_escrow;
use crate::{
    constants::seeds,
    instructions::execution::{
        __client_accounts_take_offer_buffer_accounts, __client_accounts_take_offer_config_accounts,
        __client_accounts_take_offer_custody_accounts,
        __cpi_client_accounts_take_offer_buffer_accounts,
        __cpi_client_accounts_take_offer_config_accounts,
        __cpi_client_accounts_take_offer_custody_accounts, execute, reload_managed_mint, Execution,
        ExecutionAccounts, Route, TakeOfferBufferAccounts, TakeOfferBufferAccountsBumps,
        TakeOfferConfigAccounts, TakeOfferConfigAccountsBumps, TakeOfferCustodyAccounts,
        TakeOfferCustodyAccountsBumps,
    },
    model::{seeds::FULFILLMENT_REQUEST, FulfillmentRequest, PropAmmState},
    state::State,
    utils::transfer_tokens,
    OnreError,
};
use anchor_lang::{prelude::*, AccountsClose};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[event]
pub struct FulfillmentRequestFilledEvent {
    pub fulfillment_request: Pubkey,
    pub offer_config: Pubkey,
    pub user: Pubkey,
    /// Gross input filled by this fill.
    pub input_amount: u64,
    /// Gross input filled so far, this fill included.
    pub fulfilled_input_amount: u64,
    pub fee_amount: u64,
    pub amount_out: u64,
    /// Offer Pricer's USD price applied to this fill, scale 9.
    pub price: u64,
    /// The request and its escrow were closed.
    pub fully_fulfilled: bool,
    /// Unsolicited input found in the escrow at the final fill and swept to the fee
    /// vault; zero otherwise.
    pub surplus_amount: u64,
    pub timestamp: i64,
}

/// `TakeOffer`'s execution accounts, with the request escrow as input and the
/// request user's ATA as output; the worker signs and pays for any missing fee or
/// proceeds vault ATA.
/// Permissionless offers settle directly from the escrow, which is already a
/// program account, so no intermediary is involved.
#[derive(Accounts)]
pub struct FulfillFulfillmentRequest<'info> {
    /// Offer components; must be the request's offer, enabled with its components.
    pub config: TakeOfferConfigAccounts<'info>,

    #[account(
        mut,
        seeds = [
            FULFILLMENT_REQUEST,
            fulfillment_request.offer_config.as_ref(),
            fulfillment_request.user.as_ref(),
            &fulfillment_request.request_id.to_le_bytes()
        ],
        bump = fulfillment_request.bump,
        constraint = fulfillment_request.offer_config == config.offer_config.key()
            @ OnreError::FulfillmentRequestMismatch,
        has_one = user @ OnreError::FulfillmentRequestMismatch
    )]
    pub fulfillment_request: Box<Account<'info, FulfillmentRequest>>,

    /// CHECK: The request's user, by `has_one`: receives the output and, once the
    /// request closes, its rent.
    #[account(mut)]
    pub user: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = fulfillment_request,
        associated_token::token_program = token_in_program
    )]
    pub escrow_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        address = config.offer_config.token_in_mint @ OnreError::InvalidTokenInMint,
        mint::token_program = token_in_program
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        address = config.offer_config.token_out_mint @ OnreError::InvalidTokenOutMint,
        mint::token_program = token_out_program
    )]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The user's output ATA, created at request creation. Never created here, so the
    /// worker cannot be made to fund it; if the user closed it, the fill fails and the
    /// request can be cancelled.
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = user,
        associated_token::token_program = token_out_program
    )]
    pub user_token_out: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Required exactly for asset-to-managed offers, as on `take_offer`.
    #[account(
        init_if_needed,
        payer = worker,
        associated_token::mint = token_in_mint,
        associated_token::authority = config.proceeds_vault,
        associated_token::token_program = token_in_program
    )]
    pub proceeds_token_in: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    #[account(
        init_if_needed,
        payer = worker,
        associated_token::mint = token_in_mint,
        associated_token::authority = config.fee_vault,
        associated_token::token_program = token_in_program
    )]
    pub fee_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Canonical program mint authority; mint authority is checked in the handler.
    #[account(seeds = [seeds::MINT_AUTHORITY], bump)]
    pub mint_authority: UncheckedAccount<'info>,

    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = !state.is_killed @ OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// Same BUFFER, reporting and minting accounts as `take_offer`.
    pub buffer: TakeOfferBufferAccounts<'info>,

    #[account(mut, address = state.worker @ OnreError::InvalidWorker)]
    pub worker: Signer<'info>,

    pub token_in_program: Interface<'info, TokenInterface>,
    pub token_out_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,

    /// Same pressure state and source/destination custody as `take_offer`.
    pub custody: TakeOfferCustodyAccounts<'info>,
}

impl<'info> ExecutionAccounts<'info> for FulfillFulfillmentRequest<'info> {
    fn execution(&self) -> Result<Execution<'_, 'info>> {
        Ok(Execution {
            config: &self.config,
            token_in_mint: &self.token_in_mint,
            token_out_mint: &self.token_out_mint,
            proceeds_token_in: self.proceeds_token_in.as_deref(),
            fee_token_in: &self.fee_token_in,
            mint_authority: &self.mint_authority,
            state: &self.state,
            buffer: &self.buffer,
            custody: &self.custody,
            token_in_program: &self.token_in_program,
            token_out_program: &self.token_out_program,
            associated_token_program: &self.associated_token_program,
            system_program: &self.system_program,
            payer: self.worker.as_ref(),
            user: self.user.key(),
            route: Route::Escrow {
                request: &self.fulfillment_request,
                input: &self.escrow_token_in,
                output: &self.user_token_out,
            },
        })
    }

    fn prop_amm_state_mut(&mut self) -> Option<&mut Account<'info, PropAmmState>> {
        self.custody.prop_amm_state.as_deref_mut()
    }

    fn reload_managed_mint(&mut self) -> Result<()> {
        reload_managed_mint(
            &self.config.offer_config,
            &mut self.token_in_mint,
            &mut self.token_out_mint,
        )
    }
}

/// Worker-only. Executes `input_amount` of the request's unfilled gross input
/// against its Worker offer exactly as `take_offer` would for that input: the
/// offer's quoter (NAV or Prop AMM) at the current price, its fee, and its
/// direction and custody roles, with the request user receiving the output. A full
/// fill closes the request and its escrow, refunding rent to the user; any
/// unsolicited escrow surplus goes to the fee vault. Blocked by the kill switch.
pub fn fulfill_fulfillment_request(
    ctx: Context<FulfillFulfillmentRequest>,
    input_amount: u64,
) -> Result<()> {
    require!(input_amount > 0, OnreError::InvalidAmount);
    let a = ctx.accounts;
    require!(
        input_amount <= a.fulfillment_request.remaining_input_amount(),
        OnreError::FulfillmentAmountExceedsRemaining
    );
    let quote = execute(
        ctx.program_id,
        a,
        ctx.bumps.mint_authority,
        input_amount,
        0,
        |_| Ok(()),
    )?;
    a.fulfillment_request.fulfilled_input_amount += input_amount;
    let fully_fulfilled = a.fulfillment_request.remaining_input_amount() == 0;
    let surplus = if fully_fulfilled {
        a.escrow_token_in.reload()?;
        a.escrow_token_in.amount
    } else {
        0
    };
    let request = &a.fulfillment_request;
    emit!(FulfillmentRequestFilledEvent {
        fulfillment_request: request.key(),
        offer_config: request.offer_config,
        user: request.user,
        input_amount,
        fulfilled_input_amount: request.fulfilled_input_amount,
        fee_amount: quote.fee_amount,
        amount_out: quote.amount_out,
        price: quote.price,
        fully_fulfilled,
        surplus_amount: surplus,
        timestamp: Clock::get()?.unix_timestamp,
    });
    if fully_fulfilled {
        if surplus > 0 {
            a.fulfillment_request.with_signer_seeds(|seeds| {
                transfer_tokens(
                    &a.token_in_mint,
                    &a.token_in_program,
                    &a.escrow_token_in,
                    &a.fee_token_in,
                    &a.fulfillment_request.to_account_info(),
                    Some(&[seeds]),
                    surplus,
                )
            })?;
        }
        let user = a.user.to_account_info();
        close_escrow(
            &a.fulfillment_request,
            &a.escrow_token_in,
            &user,
            &a.token_in_program,
        )?;
        a.fulfillment_request.close(user)?;
    }
    Ok(())
}
