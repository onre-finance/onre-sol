use crate::state::Offer;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, Transfer};
use anchor_spl::token_interface::TokenAccount as InterfaceTokenAccount;

/// Event emitted when an offer with one buy token is taken via permissionless route.
#[event]
pub struct OfferTakenOnePermissionless {
    pub offer_id: u64,
    pub user: Pubkey,
    pub sell_token_amount: u64,
    pub buy_token_1_amount: u64,
    pub remaining_buy_token_amount: u64,
    pub intermediary_account: Pubkey,
}

/// Account structure for taking an offer with one buy token via permissionless route.
///
/// This instruction uses intermediary token accounts controlled by the program,
/// routing both sell and buy tokens through them. The intermediary accounts persist.
///
/// # Flow
/// 1. User provides sell tokens to intermediary account
/// 2. Intermediary account transfers sell tokens to offer
/// 3. Offer transfers buy tokens to intermediary account (program-controlled)
/// 4. Intermediary account transfers buy tokens to user
///
/// # Preconditions
/// - All user ATAs must be initialized prior to execution
/// - Offer must have sufficient buy tokens available
#[derive(Accounts)]
pub struct TakeOfferOnePermissionless<'info> {
    /// The offer account being taken, providing offer details.
    /// Ensures this is a single buy token offer by checking `buy_token_mint_2`.
    #[account(
        constraint = offer.buy_token_2.mint == Pubkey::default() @ TakeOfferPermissionlessErrorCode::InvalidTakeOffer
    )]
    pub offer: Box<Account<'info, Offer>>,

    /// Offer's sell token ATA, receives the user's sell tokens.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = offer_token_authority,
    )]
    pub offer_sell_token_account: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,

    /// Offer's buy token 1 ATA, sends buy tokens to the intermediary account.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_1.mint,
        associated_token::authority = offer_token_authority,
    )]
    pub offer_buy_token_1_account: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,

    /// User's sell token ATA, sends sell tokens to the intermediary account.
    /// Ensures mint matches the offer's sell token mint.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = user,
        constraint = offer.sell_token_mint == user_sell_token_account.mint @ TakeOfferPermissionlessErrorCode::InvalidSellTokenMint
    )]
    pub user_sell_token_account: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,

    /// User's buy token 1 ATA, receives buy tokens from the intermediary account.
    /// Ensures mint matches the offer's buy token 1 mint.
    #[account(
        mut,
        associated_token::mint = buy_token_1_mint,
        associated_token::authority = user,
        constraint = offer.buy_token_1.mint == buy_token_1_mint.key() @ TakeOfferPermissionlessErrorCode::InvalidBuyTokenMint
    )]
    pub user_buy_token_1_account: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,

    /// Intermediary token account that temporarily holds buy tokens.
    /// This account is controlled by the program and must be pre-initialized.
    #[account(
        mut,
        associated_token::mint = buy_token_1_mint,
        associated_token::authority = intermediary_authority,
    )]
    pub intermediary_buy_token_account: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,

    /// Intermediary token account that temporarily holds sell tokens.
    /// This account is controlled by the program and must be pre-initialized.
    #[account(
        mut,
        associated_token::mint = sell_token_mint,
        associated_token::authority = intermediary_authority,
    )]
    pub intermediary_sell_token_account: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,

    /// The mint account for the buy token 1.
    #[account(constraint = buy_token_1_mint.key() == offer.buy_token_1.mint)]
    pub buy_token_1_mint: Box<Account<'info, Mint>>,

    /// The mint account for the sell token.
    #[account(constraint = sell_token_mint.key() == offer.sell_token_mint)]
    pub sell_token_mint: Box<Account<'info, Mint>>,

    /// Derived PDA for offer token authority, controls offer token accounts.
    /// CHECK: This account is validated by the seed derivation.
    #[account(
        seeds = [b"offer_authority", offer.offer_id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer_token_authority: AccountInfo<'info>,

    /// Derived PDA for intermediary authority, controls the intermediary token account.
    /// Uses a unique seed to avoid conflicts with other PDAs.
    /// CHECK: This account is validated by the seed derivation.
    #[account(
        seeds = [b"permissionless-1"],
        bump
    )]
    pub intermediary_authority: AccountInfo<'info>,

    /// The user taking the offer, signs the transaction and pays for account creation.
    #[account(mut)]
    pub user: Signer<'info>,

    /// SPL Token program for token operations.
    pub token_program: Program<'info, Token>,

    /// Associated Token program for ATA operations.
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// Solana System program, required for account creation.
    pub system_program: Program<'info, System>,
}

/// Calculates the current sell token amount based on the offer's dynamic pricing model.
///
/// The price of the sell token (how much is required per buy token) changes linearly over the offer's duration.
/// The offer is divided into intervals, each lasting `price_fix_duration` seconds.
/// The sell token amount starts at `sell_token_start_amount` + `one_interval_amount` at the beginning of the first interval
/// and progresses towards `sell_token_end_amount` by the end of the last interval.
///
/// # Arguments
/// - `offer`: A reference to the `Offer` account containing pricing parameters like
///   `offer_start_time`, `offer_end_time`, `price_fix_duration`,
///   `sell_token_start_amount`, and `sell_token_end_amount`.
///
/// # Returns
/// The calculated sell token amount effectively representing the "price" for the current interval.
/// This is the amount of sell tokens that corresponds to the total `buy_token_X_amount` defined in the offer for the current time interval.
///
/// # Errors
/// - [`TakeOfferPermissionlessErrorCode::InvalidCurrentTime`] if the current time is outside the offer's active period.
fn calculate_current_sell_amount(offer: &Offer) -> Result<u64> {
    let current_time = Clock::get()?.unix_timestamp as u64;

    require!(
        current_time >= offer.offer_start_time && current_time < offer.offer_end_time,
        TakeOfferPermissionlessErrorCode::InvalidCurrentTime
    );

    let total_duration = offer
        .offer_end_time
        .checked_sub(offer.offer_start_time)
        .unwrap();
    let number_of_intervals = total_duration
        .checked_div(offer.price_fix_duration)
        .unwrap();
    let current_interval = current_time
        .checked_sub(offer.offer_start_time)
        .unwrap()
        .checked_div(offer.price_fix_duration)
        .unwrap();

    let sell_token_amount_per_interval = offer
        .sell_token_end_amount
        .checked_sub(offer.sell_token_start_amount)
        .unwrap()
        .checked_div(number_of_intervals)
        .unwrap();

    let sell_token_current_amount = offer
        .sell_token_start_amount
        .checked_add(
            sell_token_amount_per_interval
                .checked_mul(current_interval + 1)
                .unwrap(),
        )
        .unwrap();

    Ok(sell_token_current_amount)
}

/// Calculates the proportional amount of a specific buy token a user receives for their sell tokens.
///
/// This function determines how many units of a particular buy token the user gets
/// based on the amount of sell tokens they provide, the total amount of that buy token
/// available in the offer, and the current effective total sell token amount required for the offer
/// (as determined by `calculate_current_sell_amount`).
///
/// Essentially, `(user_sell_token_amount / offer_sell_token_amount_at_current_price) * offer_total_buy_token_X_amount`.
///
/// # Arguments
/// - `user_sell_token_amount`: Amount of sell tokens provided by the user.
/// - `offer_buy_token_amount`: Total amount of the specific buy token set in the offer (e.g., `offer.buy_token_1.amount`).
/// - `offer_sell_token_amount`: Current effective total sell token amount for the offer at the current time interval,
///   calculated by `calculate_current_sell_amount`.
///
/// # Returns
/// The calculated amount of the specific buy token to transfer to the user, or an error if the calculation fails.
///
/// # Errors
/// - [`TakeOfferPermissionlessErrorCode::InvalidSellTokenMint`] if `offer_sell_token_amount` (the denominator) is zero.
/// - [`TakeOfferPermissionlessErrorCode::CalculationOverflow`] if multiplication or division results in overflow.
/// - [`TakeOfferPermissionlessErrorCode::ZeroBuyTokenAmount`] if the calculated buy token amount for the user is zero.
fn calculate_buy_amount(
    user_sell_token_amount: u64,
    offer_buy_token_amount: u64,
    offer_sell_token_amount: u64,
) -> Result<u64> {
    if offer_sell_token_amount == 0 {
        return Err(error!(TakeOfferPermissionlessErrorCode::InvalidSellTokenMint).into());
    }
    let result = (user_sell_token_amount as u128)
        .checked_mul(offer_buy_token_amount as u128)
        .ok_or(TakeOfferPermissionlessErrorCode::CalculationOverflow)?
        .checked_div(offer_sell_token_amount as u128)
        .ok_or(TakeOfferPermissionlessErrorCode::CalculationOverflow)?;
    if result > u64::MAX as u128 {
        return Err(error!(
            TakeOfferPermissionlessErrorCode::CalculationOverflow
        ));
    }
    if result == 0 {
        return Err(error!(TakeOfferPermissionlessErrorCode::ZeroBuyTokenAmount));
    }
    Ok(result as u64)
}

/// Takes an offer with one buy token via permissionless route.
///
/// Uses an intermediary account to route tokens through it. The intermediary account persists.
/// This provides an additional layer of indirection while maintaining the same economic outcome.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the offer take.
/// - `sell_token_amount`: Amount of sell tokens the user provides to exchange for buy tokens.
///
/// # Errors
/// - [`TakeOfferPermissionlessErrorCode::InvalidCurrentTime`] if the offer is not active.
/// - [`TakeOfferPermissionlessErrorCode::InsufficientOfferTokenOneBalance`] if the offer lacks sufficient buy tokens.
/// - [`TakeOfferPermissionlessErrorCode::CalculationOverflow`] if amount calculations overflow.
/// - [`TakeOfferPermissionlessErrorCode::ZeroBuyTokenAmount`] if the calculated buy token amount is zero.
pub fn take_offer_one_permissionless(
    ctx: Context<TakeOfferOnePermissionless>,
    sell_token_amount: u64,
) -> Result<()> {
    let offer = &ctx.accounts.offer;

    let current_sell_token_amount = calculate_current_sell_amount(&offer).unwrap();
    msg!(
        "Calculated current sell token amount: {}",
        current_sell_token_amount
    );

    let buy_token_1_amount = calculate_buy_amount(
        sell_token_amount,
        offer.buy_token_1.amount,
        current_sell_token_amount,
    )?;

    msg!("Calculated buy token 1 amount: {}", buy_token_1_amount);
    require!(
        ctx.accounts.offer_buy_token_1_account.amount >= buy_token_1_amount,
        TakeOfferPermissionlessErrorCode::InsufficientOfferTokenOneBalance
    );

    // Step 1: Transfer sell tokens from user to intermediary account
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_sell_token_account.to_account_info(),
                to: ctx
                    .accounts
                    .intermediary_sell_token_account
                    .to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        sell_token_amount,
    )?;
    msg!(
        "Transferring {} sell tokens from user to intermediary",
        sell_token_amount
    );

    // Step 2: Transfer sell tokens from intermediary account to offer
    let intermediary_seeds = &[
        b"permissionless-1".as_ref(),
        &[ctx.bumps.intermediary_authority],
    ];
    let intermediary_signer_seeds = &[&intermediary_seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx
                    .accounts
                    .intermediary_sell_token_account
                    .to_account_info(),
                to: ctx.accounts.offer_sell_token_account.to_account_info(),
                authority: ctx.accounts.intermediary_authority.to_account_info(),
            },
            intermediary_signer_seeds,
        ),
        sell_token_amount,
    )?;
    msg!(
        "Transferring {} sell tokens from intermediary to offer",
        sell_token_amount
    );

    // Step 3: Transfer buy tokens from offer to intermediary account
    let offer_id_bytes = &offer.offer_id.to_le_bytes();
    let offer_seeds = &[
        b"offer_authority".as_ref(),
        offer_id_bytes,
        &[offer.authority_bump],
    ];
    let offer_signer_seeds = &[&offer_seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.offer_buy_token_1_account.to_account_info(),
                to: ctx
                    .accounts
                    .intermediary_buy_token_account
                    .to_account_info(),
                authority: ctx.accounts.offer_token_authority.to_account_info(),
            },
            offer_signer_seeds,
        ),
        buy_token_1_amount,
    )?;
    msg!(
        "Transferring {} buy tokens from offer to intermediary account",
        buy_token_1_amount
    );

    // Step 4: Transfer buy tokens from intermediary account to user
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx
                    .accounts
                    .intermediary_buy_token_account
                    .to_account_info(),
                to: ctx.accounts.user_buy_token_1_account.to_account_info(),
                authority: ctx.accounts.intermediary_authority.to_account_info(),
            },
            intermediary_signer_seeds,
        ),
        buy_token_1_amount,
    )?;
    msg!(
        "Transferring {} buy tokens from intermediary to user",
        buy_token_1_amount
    );

    emit!(OfferTakenOnePermissionless {
        offer_id: offer.offer_id,
        user: ctx.accounts.user.key(),
        sell_token_amount,
        buy_token_1_amount,
        remaining_buy_token_amount: ctx.accounts.offer_buy_token_1_account.amount
            - buy_token_1_amount,
        intermediary_account: ctx.accounts.intermediary_buy_token_account.key(),
    });

    Ok(())
}

/// Error codes for permissionless offer taking operations.
#[error_code]
pub enum TakeOfferPermissionlessErrorCode {
    /// Triggered when the offer lacks sufficient buy tokens to fulfill the take.
    #[msg("Insufficient tokens remaining in the offer for token 1.")]
    InsufficientOfferTokenOneBalance,

    /// Triggered when the user's sell token mint doesn't match the offer's.
    #[msg("The sell token mint does not match the offer.")]
    InvalidSellTokenMint,

    /// Triggered when the user's buy token mint doesn't match the offer's.
    #[msg("The buy token mint does not match the offer.")]
    InvalidBuyTokenMint,

    /// Triggered when the offer type is invalid for the take instruction.
    #[msg("The offer is of 2 buy token type.")]
    InvalidTakeOffer,

    /// Triggered when buy amount calculations overflow or are invalid.
    #[msg("Calculation overflowed or invalid.")]
    CalculationOverflow,

    /// Triggered when the calculated buy token amount is zero.
    #[msg("Zero buy token amount.")]
    ZeroBuyTokenAmount,

    /// Triggered when the current time is outside the offer's time range.
    #[msg("Current time must be within the offer's start and end time range.")]
    InvalidCurrentTime,
}
