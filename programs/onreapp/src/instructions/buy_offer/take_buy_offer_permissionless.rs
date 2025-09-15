use crate::constants::seeds;
use crate::instructions::buy_offer::buy_offer_utils::process_buy_offer_core;
use crate::instructions::{find_offer, BuyOfferAccount};
use crate::state::State;
use crate::utils::{execute_token_operations, transfer_tokens, u64_to_dec9, ExecTokenOpsParams};
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

/// Error codes specific to the take_buy_offer_permissionless instruction
#[error_code]
pub enum TakeBuyOfferPermissionlessErrorCode {
    #[msg("Invalid boss account")]
    InvalidBoss,
}

/// Event emitted when a buy offer is successfully taken via permissionless flow
#[event]
pub struct TakeBuyOfferPermissionlessEvent {
    /// The ID of the buy offer that was taken
    pub offer_id: u64,
    /// Amount of token_in paid by the user
    pub token_in_amount: u64,
    /// Amount of token_out received by the user
    pub token_out_amount: u64,
    /// Fee amount paid by the user in token_in
    pub fee_amount: u64,
    /// Public key of the user who took the offer
    pub user: Pubkey,
}

/// Accounts required for taking a buy offer via permissionless flow
///
/// This struct defines all the accounts needed to execute a take_buy_offer_permissionless instruction,
/// including intermediary accounts owned by the program for routing token transfers.
#[derive(Accounts)]
pub struct TakeBuyOfferPermissionless<'info> {
    /// The buy offer account containing all active buy offers
    #[account(mut, seeds = [seeds::BUY_OFFERS], bump)]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

    /// Program state account containing the boss public key
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// The boss account that receives token_in payments
    /// Must match the boss stored in the program state
    /// CHECK: This account is validated through the constraint above
    pub boss: UncheckedAccount<'info>,

    /// The buy offer vault authority PDA that controls vault token accounts
    #[account(seeds = [seeds::BUY_OFFER_VAULT_AUTHORITY], bump)]
    /// CHECK: This is safe as it's a PDA used for signing
    pub vault_authority: UncheckedAccount<'info>,

    /// Vault's token_in account, used for burning tokens when program has mint authority
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = vault_authority
    )]
    pub vault_token_in_account: Account<'info, TokenAccount>,

    /// Vault's token_out account (source of tokens to distribute, when program doesn't have mint authority)
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = vault_authority
    )]
    pub vault_token_out_account: Account<'info, TokenAccount>,

    /// The permissionless authority PDA that controls intermediary token accounts
    #[account(seeds = [seeds::PERMISSIONLESS_1], bump)]
    /// CHECK: This is safe as it's a PDA used for signing
    pub permissionless_authority: UncheckedAccount<'info>,

    /// Permissionless intermediary token_in account (temporary holding for token_in)
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = permissionless_authority
    )]
    pub permissionless_token_in_account: Account<'info, TokenAccount>,

    /// Permissionless intermediary token_out account (temporary holding for token_out)
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = permissionless_authority
    )]
    pub permissionless_token_out_account: Account<'info, TokenAccount>,

    /// The mint account for the input token (what user pays)
    pub token_in_mint: Account<'info, Mint>,

    /// The mint account for the output token (what user receives)
    /// Must be mutable to allow minting when program has mint authority
    #[account(mut)]
    pub token_out_mint: Account<'info, Mint>,

    /// User's token_in account (source of payment)
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = user
    )]
    pub user_token_in_account: Account<'info, TokenAccount>,

    /// User's token_out account (destination of received tokens)
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_out_mint,
        associated_token::authority = user
    )]
    pub user_token_out_account: Account<'info, TokenAccount>,

    /// Boss's token_in account (final destination of user's payment)
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = boss
    )]
    pub boss_token_in_account: Account<'info, TokenAccount>,

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

    /// SPL Token program for token transfers
    pub token_program: Program<'info, Token>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program required for account creation
    pub system_program: Program<'info, System>,
}

/// Main instruction handler for taking a buy offer via permissionless flow
///
/// This function allows users to accept a buy offer using intermediary accounts owned by the program.
/// Instead of direct transfers, tokens are routed through permissionless intermediary accounts:
/// 1. User → Permissionless intermediary (token_in)
/// 2. Permissionless intermediary → Boss (token_in)
/// 3. Vault → Permissionless intermediary (token_out)
/// 4. Permissionless intermediary → User (token_out)
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
/// 5. Execute atomic transfers through intermediary accounts
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
pub fn take_buy_offer_permissionless(
    ctx: Context<TakeBuyOfferPermissionless>,
    offer_id: u64,
    token_in_amount: u64,
) -> Result<()> {
    let offer_account = ctx.accounts.buy_offer_account.load_mut()?;

    // Find the offer to get fee information
    let offer = find_offer(&offer_account, offer_id)?;

    // Use shared core processing logic
    let result = process_buy_offer_core(
        &offer_account,
        offer_id,
        token_in_amount,
        offer.fee_basis_points,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
    )?;

    // 1. Transfer token_in from user to permissionless intermediary
    transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.user_token_in_account,
        &ctx.accounts.permissionless_token_in_account,
        &ctx.accounts.user,
        None,
        token_in_amount,
    )?;
    msg!("Transferred token_in from user to permissionless intermediary");

    // 2. Execute token operations (transfer + burn for token_in, transfer for token_out)
    execute_token_operations(ExecTokenOpsParams {
        token_program: &ctx.accounts.token_program,
        // Token in params
        token_in_mint: &ctx.accounts.token_in_mint,
        token_in_amount: result.token_in_amount,
        token_in_authority: &ctx.accounts.permissionless_authority,
        token_in_source_signer_seeds: Some(&[&[
            seeds::PERMISSIONLESS_1,
            &[ctx.bumps.permissionless_authority],
        ]]),
        token_in_burn_signer_seeds: Some(&[&[
            seeds::BUY_OFFER_VAULT_AUTHORITY,
            &[ctx.bumps.vault_authority],
        ]]),
        token_in_source_account: &ctx.accounts.permissionless_token_in_account,
        token_in_destination_account: &ctx.accounts.boss_token_in_account,
        token_in_burn_account: &ctx.accounts.vault_token_in_account,
        token_in_burn_authority: &ctx.accounts.vault_authority,
        // Token out params
        token_out_mint: &ctx.accounts.token_out_mint,
        token_out_amount: result.token_out_amount,
        token_out_authority: &ctx.accounts.vault_authority,
        token_out_source_account: &ctx.accounts.vault_token_out_account,
        token_out_destination_account: &ctx.accounts.permissionless_token_out_account,
        token_out_signer_seeds: Some(&[&[
            seeds::BUY_OFFER_VAULT_AUTHORITY,
            &[ctx.bumps.vault_authority],
        ]]),
        mint_authority_pda: &ctx.accounts.mint_authority_pda,
        mint_authority_bump: &[ctx.bumps.mint_authority_pda],
    })?;

    // 3. Transfer token_out from permissionless intermediary to user
    let permissionless_authority_seeds = &[
        seeds::PERMISSIONLESS_1,
        &[ctx.bumps.permissionless_authority],
    ];
    let permissionless_signer_seeds = &[permissionless_authority_seeds.as_slice()];

    transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.permissionless_token_out_account,
        &ctx.accounts.user_token_out_account,
        &ctx.accounts.permissionless_authority,
        Some(permissionless_signer_seeds),
        result.token_out_amount,
    )?;

    msg!(
        "Buy offer taken (permissionless) - ID: {}, token_in(excluding fee): {}, fee: {}, token_out: {}, user: {}, price: {}",
        offer_id,
        result.token_in_amount,
        result.fee_amount,
        result.token_out_amount,
        ctx.accounts.user.key,
        u64_to_dec9(result.current_price)
    );

    emit!(TakeBuyOfferPermissionlessEvent {
        offer_id,
        token_in_amount: result.token_in_amount,
        token_out_amount: result.token_out_amount,
        fee_amount: result.fee_amount,
        user: ctx.accounts.user.key(),
    });

    Ok(())
}
