//! User escrow of a Worker-execution offer's input.
use crate::{
    constants::seeds::STATE,
    instructions::execution::{
        __client_accounts_take_offer_config_accounts,
        __cpi_client_accounts_take_offer_config_accounts, authorize, TakeOfferConfigAccounts,
        TakeOfferConfigAccountsBumps,
    },
    model::{seeds::FULFILLMENT_REQUEST, FulfillmentRequest, OfferExecution},
    state::State,
    utils::{has_transfer_fee, transfer_tokens, ApprovalMessage},
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use solana_instructions_sysvar::ID as INSTRUCTIONS_SYSVAR_ID;

#[event]
pub struct FulfillmentRequestedEvent {
    pub fulfillment_request: Pubkey,
    pub offer_config: Pubkey,
    pub user: Pubkey,
    pub request_id: u64,
    pub input_amount: u64,
    pub timestamp: i64,
}

#[derive(Accounts)]
#[instruction(request_id: u64)]
pub struct CreateFulfillmentRequest<'info> {
    /// The offer and every component it uses must be enabled, as on execution.
    pub config: TakeOfferConfigAccounts<'info>,

    #[account(
        address = config.offer_config.token_in_mint @ OnreError::InvalidTokenInMint,
        mint::token_program = token_in_program,
        constraint = config.offer_config.execution == OfferExecution::Worker @ OnreError::UnsupportedOfferExecution
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = user,
        associated_token::token_program = token_in_program
    )]
    pub user_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Fails if the user already has an open request with this ID on this offer.
    #[account(
        init,
        payer = user,
        space = 8 + FulfillmentRequest::INIT_SPACE,
        seeds = [
            FULFILLMENT_REQUEST,
            config.offer_config.key().as_ref(),
            user.key().as_ref(),
            &request_id.to_le_bytes()
        ],
        bump
    )]
    pub fulfillment_request: Box<Account<'info, FulfillmentRequest>>,

    /// The request's input-mint ATA, holding the escrowed input until filled. Accepted
    /// if someone created it first, so its predictable address cannot block the ID.
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_in_mint,
        associated_token::authority = fulfillment_request,
        associated_token::token_program = token_in_program
    )]
    pub escrow_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        address = config.offer_config.token_out_mint @ OnreError::InvalidTokenOutMint,
        mint::token_program = token_out_program
    )]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The user's output ATA, created here at the user's expense so fills never pay
    /// its rent: the worker would otherwise fund a new account on every partial fill
    /// after the user closes it.
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_out_mint,
        associated_token::authority = user,
        associated_token::token_program = token_out_program
    )]
    pub user_token_out: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        seeds = [STATE],
        bump = state.bump,
        constraint = !state.is_killed @ OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// CHECK: Ed25519 approval verification reads the canonical instructions sysvar.
    #[account(address = INSTRUCTIONS_SYSVAR_ID @ OnreError::InvalidInstructionsSysvar)]
    pub instructions_sysvar: UncheckedAccount<'info>,

    /// Pays the request and escrow rent, refunded when the request closes, and the
    /// output ATA if missing.
    #[account(mut)]
    pub user: Signer<'info>,

    pub token_in_program: Interface<'info, TokenInterface>,
    pub token_out_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

/// Escrows `input_amount` of the offer's input for the worker to fill; locks no
/// price or fee. Permissioned offers require an approver-signed approval for the
/// user, permissionless offers reject one. Blocked by the kill switch.
pub fn create_fulfillment_request(
    ctx: Context<CreateFulfillmentRequest>,
    request_id: u64,
    input_amount: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let a = ctx.accounts;
    let offer_config = a.config.offer_config.key();
    require!(input_amount > 0, OnreError::InvalidAmount);
    require!(
        !has_transfer_fee(&a.token_in_mint)?,
        OnreError::TransferFeeNotSupported
    );
    authorize(
        ctx.program_id,
        a.config.offer_config.access,
        &a.user.key(),
        &a.state,
        &a.instructions_sysvar,
        approval_message.as_ref(),
    )?;
    transfer_tokens(
        &a.token_in_mint,
        &a.token_in_program,
        &a.user_token_in,
        &a.escrow_token_in,
        &a.user.to_account_info(),
        None,
        input_amount,
    )?;
    a.fulfillment_request.set_inner(FulfillmentRequest {
        offer_config,
        user: a.user.key(),
        request_id,
        input_amount,
        fulfilled_input_amount: 0,
        bump: ctx.bumps.fulfillment_request,
    });
    emit!(FulfillmentRequestedEvent {
        fulfillment_request: a.fulfillment_request.key(),
        offer_config,
        user: a.user.key(),
        request_id,
        input_amount,
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
