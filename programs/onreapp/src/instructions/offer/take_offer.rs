use crate::constants::seeds;
use crate::instructions::offer::offer_utils::process_offer_core;
use crate::instructions::OfferAccount;
use crate::state::State;
use crate::utils::{execute_token_operations, u64_to_dec9, ExecTokenOpsParams};
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Error codes specific to the take_offer instruction
#[error_code]
pub enum TakeOfferErrorCode {
    #[msg("Invalid boss account")]
    InvalidBoss,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Kill switch is activated")]
    KillSwitchActivated,
}

/// Event emitted when an offer is successfully taken
#[event]
pub struct TakeOfferEvent {
    /// The ID of the offer that was taken
    pub offer_id: u64,
    /// Amount of token_in paid by the user (excluding fee)
    pub token_in_amount: u64,
    /// Amount of token_out received by the user
    pub token_out_amount: u64,
    /// Fee amount paid by the user in token_in
    pub fee_amount: u64,
    /// Public key of the user who took the offer
    pub user: Pubkey,
}

/// Accounts required for taking an offer
///
/// This struct defines all the accounts needed to execute a take_offer instruction,
/// including validation constraints to ensure security and proper authorization.
#[derive(Accounts)]
pub struct TakeOffer<'info> {
    /// The offer account containing all active offers
    #[account(mut, seeds = [seeds::OFFERS], bump)]
    pub offer_account: AccountLoader<'info, OfferAccount>,

    /// Program state account containing the boss public key
    #[account(
        constraint = state.is_killed == false @ TakeOfferErrorCode::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// The boss account that receives token_in payments
    /// Must match the boss stored in the program state
    #[account(
        constraint = boss.key() == state.boss @ TakeOfferErrorCode::InvalidBoss
    )]
    /// CHECK: This account is validated through the constraint above
    pub boss: UncheckedAccount<'info>,

    /// The offer vault authority PDA that controls vault token accounts
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    /// CHECK: This is safe as it's a PDA used for signing
    pub vault_authority: UncheckedAccount<'info>,

    /// Vault's token_in account, used for burning tokens when program has mint authority
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_in_program
    )]
    pub vault_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Vault's token_out account (source of tokens to distribute to user)
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_out_program
    )]
    pub vault_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The mint account for the input token (what user pays)
    /// Must be mutable to allow burning when program has mint authority
    #[account(mut)]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_in_program: Interface<'info, TokenInterface>,

    /// The mint account for the output token (what user receives)
    /// Must be mutable to allow minting when program has mint authority
    #[account(mut)]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_out_program: Interface<'info, TokenInterface>,

    /// User's token_in account (source of payment)
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = user,
        associated_token::token_program = token_in_program
    )]
    pub user_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User's token_out account (destination of received tokens)
    /// Uses init_if_needed to automatically create account if it doesn't exist
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_out_mint,
        associated_token::authority = user,
        associated_token::token_program = token_out_program
    )]
    pub user_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Boss's token_in account (destination of user's payment)
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_in_program
    )]
    pub boss_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Mint authority PDA for direct minting (when program has mint authority)
    /// CHECK: PDA derivation is validated through seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        bump
    )]
    pub mint_authority_pda: UncheckedAccount<'info>,

    /// The user taking the offer (must sign the transaction)
    #[account(mut)]
    pub user: Signer<'info>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program required for account creation
    pub system_program: Program<'info, System>,
}

/// Main instruction handler for taking an offer with mint/transfer flexibility
///
/// This function allows users to accept an offer by paying the current market price
/// in token_in to receive token_out. The price is calculated using a discrete interval
/// pricing model with linear yield growth. Token distribution uses smart logic:
/// - If program has mint authority: mints token_out directly to user (more efficient)
/// - If program lacks mint authority: transfers token_out from vault to user (fallback)
///
/// # Arguments
///
/// * `ctx` - The instruction context containing all required accounts
/// * `offer_id` - The unique ID of the offer to take
/// * `token_in_amount` - The amount of token_in the user is willing to pay
///
/// # Process Flow
///
/// 1. Load and validate the offer exists
/// 2. Find the currently active pricing vector
/// 3. Calculate current price based on time elapsed and APR parameters
/// 4. Calculate how many token_out to give for the provided token_in_amount
/// 5. Execute payment: user → boss (token_in)
/// 6. Execute distribution: program mints token_out to user OR transfers from vault
/// 7. Emit event with transaction details
///
/// # Account Requirements
///
/// * `mint_authority_pda` - Optional, required only if program should mint directly
/// * `vault_token_out_account` - Optional, required only if program should transfer from vault
/// * At least one of the above must be provided for token distribution
///
/// # Returns
///
/// * `Ok(())` - If the offer was successfully taken
/// * `Err(_)` - If validation fails or token operations cannot be completed
///
/// # Errors
///
/// * `OfferNotFound` - The specified offer_id doesn't exist
/// * `NoActiveVector` - No pricing vector is currently active  
/// * `OverflowError` - Mathematical overflow in price calculations
/// * Token operation errors if insufficient balances, invalid accounts, or missing mint authority
pub fn take_offer(ctx: Context<TakeOffer>, offer_id: u64, token_in_amount: u64) -> Result<()> {
    let offer_account = ctx.accounts.offer_account.load()?;

    // Use shared core processing logic for main exchange amount
    let result = process_offer_core(
        &offer_account,
        offer_id,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
    )?;

    execute_token_operations(ExecTokenOpsParams {
        // Token in params
        token_in_program: &ctx.accounts.token_in_program,
        token_in_mint: &ctx.accounts.token_in_mint,
        token_in_amount, // Including fee
        token_in_authority: &ctx.accounts.user,
        token_in_source_signer_seeds: None,
        vault_authority_signer_seeds: Some(&[&[
            seeds::OFFER_VAULT_AUTHORITY,
            &[ctx.bumps.vault_authority],
        ]]),
        token_in_source_account: &ctx.accounts.user_token_in_account,
        token_in_destination_account: &ctx.accounts.boss_token_in_account,
        token_in_burn_account: &ctx.accounts.vault_token_in_account,
        token_in_burn_authority: &ctx.accounts.vault_authority,
        // Token out params
        token_out_program: &ctx.accounts.token_out_program,
        token_out_mint: &ctx.accounts.token_out_mint,
        token_out_amount: result.token_out_amount,
        token_out_authority: &ctx.accounts.vault_authority,
        token_out_source_account: &ctx.accounts.vault_token_out_account,
        token_out_destination_account: &ctx.accounts.user_token_out_account,
        mint_authority_pda: &ctx.accounts.mint_authority_pda,
        mint_authority_bump: &[ctx.bumps.mint_authority_pda],
    })?;

    msg!(
        "Offer taken - ID: {}, token_in(+fee): {}(+{}), token_out: {}, user: {}, price: {}",
        offer_id,
        result.token_in_amount,
        result.fee_amount,
        result.token_out_amount,
        ctx.accounts.user.key,
        u64_to_dec9(result.current_price)
    );

    emit!(TakeOfferEvent {
        offer_id,
        token_in_amount: result.token_in_amount,
        token_out_amount: result.token_out_amount,
        fee_amount: result.fee_amount,
        user: ctx.accounts.user.key(),
    });

    Ok(())
}
