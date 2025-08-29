use crate::constants::seeds;
use crate::instructions::buy_offer::buy_offer_utils::{find_offer, process_buy_offer_core};
use crate::instructions::{execute_direct_transfers, BuyOfferAccount};
use crate::state::State;
use crate::utils::{calculate_fees, u64_to_dec9};
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token::{Mint, Token, TokenAccount};

/// Error codes specific to the take_buy_offer instruction
#[error_code]
pub enum TakeBuyOfferErrorCode {
    #[msg("Invalid boss account")]
    InvalidBoss,
    #[msg("Math overflow")]
    MathOverflow,
}

/// Event emitted when a buy offer is successfully taken
#[event]
pub struct TakeBuyOfferEvent {
    /// The ID of the buy offer that was taken
    pub offer_id: u64,
    /// Amount of token_in paid by the user (including fee)
    pub token_in_amount: u64,
    /// Amount of token_out received by the user
    pub token_out_amount: u64,
    /// Fee amount paid by the user in token_in
    pub fee_amount: u64,
    /// Public key of the user who took the offer
    pub user: Pubkey,
}

/// Accounts required for taking a buy offer
///
/// This struct defines all the accounts needed to execute a take_buy_offer instruction,
/// including validation constraints to ensure security and proper authorization.
#[derive(Accounts)]
pub struct TakeBuyOffer<'info> {
    /// The buy offer account containing all active buy offers
    #[account(mut, seeds = [seeds::BUY_OFFERS], bump)]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

    /// Program state account containing the boss public key
    pub state: Box<Account<'info, State>>,

    /// The boss account that receives token_in payments
    /// Must match the boss stored in the program state
    #[account(
        constraint = boss.key() == state.boss @ TakeBuyOfferErrorCode::InvalidBoss
    )]
    /// CHECK: This account is validated through the constraint above
    pub boss: UncheckedAccount<'info>,

    /// The buy offer vault authority PDA that controls vault token accounts
    #[account(seeds = [seeds::BUY_OFFER_VAULT_AUTHORITY], bump)]
    /// CHECK: This is safe as it's a PDA used for signing
    pub vault_authority: UncheckedAccount<'info>,

    /// The mint account for the input token (what user pays)
    pub token_in_mint: Box<Account<'info, Mint>>,

    /// The mint account for the output token (what user receives)
    pub token_out_mint: Box<Account<'info, Mint>>,

    /// User's token_in account (source of payment)
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = user
    )]
    pub user_token_in_account: Box<Account<'info, TokenAccount>>,

    /// User's token_out account (destination of received tokens)
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = user
    )]
    pub user_token_out_account: Box<Account<'info, TokenAccount>>,

    /// Boss's token_in account (destination of user's payment)
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = boss
    )]
    pub boss_token_in_account: Box<Account<'info, TokenAccount>>,

    /// Vault's token_out account (source of tokens to distribute to user)
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = vault_authority
    )]
    pub vault_token_out_account: Box<Account<'info, TokenAccount>>,

    /// The user taking the offer (must sign the transaction)
    #[account(mut)]
    pub user: Signer<'info>,

    /// SPL Token program for token transfers
    pub token_program: Program<'info, Token>,
}

/// Main instruction handler for taking a buy offer
///
/// This function allows users to accept a buy offer by paying the current market price
/// in token_in to receive token_out from the vault. The price is calculated using a
/// discrete interval pricing model with linear yield growth.
///
/// # Arguments
///
/// * `ctx` - The instruction context containing all required accounts
/// * `offer_id` - The unique ID of the buy offer to take
/// * `token_in_amount` - The amount of token_in the user is willing to pay
///
/// # Process Flow
///
/// 1. Load and validate the buy offer exists
/// 2. Find the currently active pricing vector
/// 3. Calculate current price based on time elapsed and APR parameters
/// 4. Calculate how many token_out to give for the provided token_in_amount
/// 5. Execute atomic transfers: user → boss (token_in), vault → user (token_out)
/// 6. Emit event with transaction details
///
/// # Returns
///
/// * `Ok(())` - If the offer was successfully taken
/// * `Err(_)` - If validation fails or transfers cannot be completed
///
/// # Errors
///
/// * `OfferNotFound` - The specified offer_id doesn't exist
/// * `NoActiveVector` - No pricing vector is currently active  
/// * `OverflowError` - Mathematical overflow in price calculations
/// * Token transfer errors if insufficient balances or invalid accounts
pub fn take_buy_offer(
    ctx: Context<TakeBuyOffer>,
    offer_id: u64,
    token_in_amount: u64,
) -> Result<()> {
    let offer_account = ctx.accounts.buy_offer_account.load()?;

    // Find the offer to get fee information
    let offer = find_offer(&offer_account, offer_id)?;

    let fee_amounts = calculate_fees(token_in_amount, offer.fee_basis_points)?;

    // Use shared core processing logic for main exchange amount
    let result = process_buy_offer_core(
        &offer_account,
        offer_id,
        fee_amounts.remaining_token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
    )?;

    execute_direct_transfers(
        &ctx.accounts.user,
        &ctx.accounts.user_token_in_account,
        &ctx.accounts.boss_token_in_account,
        &ctx.accounts.vault_authority,
        &ctx.accounts.vault_token_out_account,
        &ctx.accounts.user_token_out_account,
        &ctx.accounts.token_program,
        ctx.bumps.vault_authority,
        fee_amounts.remaining_token_in_amount,
        result.token_out_amount,
    )?;

    msg!(
        "Buy offer taken - ID: {}, token_in: {}, fee: {}, token_out: {}, user: {}, price: {}",
        offer_id,
        token_in_amount,
        fee_amounts.fee_amount,
        result.token_out_amount,
        ctx.accounts.user.key,
        u64_to_dec9(result.current_price)
    );

    emit!(TakeBuyOfferEvent {
        offer_id,
        token_in_amount,
        token_out_amount: result.token_out_amount,
        fee_amount: fee_amounts.fee_amount,
        user: ctx.accounts.user.key(),
    });

    Ok(())
}
