use crate::state::Offer;
use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

/// Event emitted when an offer with one buy token is taken.
#[event]
pub struct OfferTakenOne {
    pub offer_id: u64,
    pub user: Pubkey,
    pub sell_token_amount: u64,
    pub buy_token_1_amount: u64,
    pub remaining_sell_token_amount: u64,
}

/// Event emitted when an offer with two buy tokens is taken.
#[event]
pub struct OfferTakenTwo {
    pub offer_id: u64,
    pub user: Pubkey,
    pub sell_token_amount: u64,
    pub buy_token_1_amount: u64,
    pub buy_token_2_amount: u64,
    pub remaining_sell_token_amount: u64,
}

/// Account structure for taking an offer with one buy token.
///
/// This struct defines the accounts required to accept an offer, exchanging sell tokens
/// for one buy token from the offer.
///
/// # Preconditions
/// - All Associated Token Accounts (ATAs) must be initialized prior to execution.
///   This includes `offer_sell_token_account`, `offer_buy_token_1_account`,
///   `user_sell_token_account`, and `user_buy_token_1_account`.
#[derive(Accounts)]
pub struct TakeOfferOne<'info> {
    /// The offer account being taken, providing offer details.
    /// Ensures this is a single buy token offer by checking `buy_token_mint_2`.
    #[account(
        constraint = offer.buy_token_2.mint == system_program::ID @ TakeOfferErrorCode::InvalidTakeOffer
    )]
    pub offer: Account<'info, Offer>,

    /// Offer's sell token ATA, receives the user's sell tokens.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = offer_token_authority,
    )]
    pub offer_sell_token_account: Account<'info, TokenAccount>,

    /// Offer's buy token 1 ATA, sends buy tokens to the user.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_1.mint,
        associated_token::authority = offer_token_authority,
    )]
    pub offer_buy_token_1_account: Account<'info, TokenAccount>,

    /// User's sell token ATA, sends sell tokens to the offer.
    /// Ensures mint matches the offer's sell token mint.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = user,
        constraint = offer.sell_token_mint == user_sell_token_account.mint @ TakeOfferErrorCode::InvalidSellTokenMint
    )]
    pub user_sell_token_account: Account<'info, TokenAccount>,

    /// User's buy token 1 ATA, receives buy tokens from the offer.
    /// Ensures mint matches the offer's buy token 1 mint.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_1.mint,
        associated_token::authority = user,
        constraint = offer.buy_token_1.mint == user_buy_token_1_account.mint @ TakeOfferErrorCode::InvalidBuyTokenMint
    )]
    pub user_buy_token_1_account: Account<'info, TokenAccount>,

    /// Derived PDA for token authority, controls offer token accounts.
    ///
    /// # Note
    /// This account is marked with `CHECK` as it's validated by the seed derivation.
    #[account(
        seeds = [b"offer_authority", offer.offer_id.to_le_bytes().as_ref()],
        bump
  )]
    pub offer_token_authority: AccountInfo<'info>,

    /// The user taking the offer, signs the transaction.
    #[account(mut)]
    pub user: Signer<'info>,

    /// SPL Token program for token operations.
    pub token_program: Program<'info, Token>,

    /// Solana System program, included for potential rent accounting.
    pub system_program: Program<'info, System>,
}

/// Calculates the received buy token amount based on time sectors and user's sell token amount.
///
/// # Arguments
/// - `current_time`: Current timestamp
/// - `offer_start_time`: Start time of the offer
/// - `offer_end_time`: End time of the offer
/// - `price_fix_duration`: Duration of each price fix period in seconds
/// - `sell_token_start_amount`: Initial sell token amount
/// - `sell_token_end_amount`: Final sell token amount
/// - `user_sell_token_amount`: Amount of tokens the user wants to exchange
///
/// # Returns
/// The calculated sell token amount for the current time
fn calculate_current_sell_amount(
    offer: &Offer
) -> Result<u64> {
    let current_time = Clock::get()?.unix_timestamp as u64;
    require!(
        current_time >= offer.offer_start_time && current_time <= offer.offer_end_time,
        TakeOfferErrorCode::InvalidCurrentTime
    );

    let total_duration = offer.offer_end_time.checked_sub(offer.offer_start_time).unwrap();
    let number_of_intervals = total_duration.checked_div(offer.price_fix_duration).unwrap();
    let current_interval = current_time
        .checked_sub(offer.offer_start_time)
        .unwrap()
        .checked_div(offer.price_fix_duration)
        .unwrap();

    let sell_token_amount_per_interval = offer.sell_token_end_amount
        .checked_sub(offer.sell_token_start_amount)
        .unwrap()
        .checked_div(number_of_intervals)
        .unwrap();

    let sell_token_current_amount = offer.sell_token_start_amount
        .checked_add(sell_token_amount_per_interval.checked_mul(current_interval + 1).unwrap()) // TODO: Check if bucket shift is required
        .unwrap();

    Ok(sell_token_current_amount)
}

/// Takes an offer with one buy token.
///
/// Allows a user to exchange sell tokens for one buy token from the offer, transferring
/// tokens between accounts and emitting an `OfferTakenOne` event with transaction details.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the offer take.
/// - `sell_token_amount`: Amount of sell tokens the user provides.
///
/// # Errors
/// - [`TakeOfferErrorCode::OfferExceedsSellLimit`] if the sell amount exceeds the offer's limit.
/// - [`TakeOfferErrorCode::InsufficientOfferBalance`] if the offer lacks sufficient buy tokens.
/// - [`TakeOfferErrorCode::CalculationOverflow`] if amount calculations overflow.
pub fn take_offer_one(ctx: Context<TakeOfferOne>, sell_token_amount: u64) -> Result<()> {
    let offer = &ctx.accounts.offer;

    require!(
        ctx.accounts
            .offer_sell_token_account
            .amount
            .checked_add(sell_token_amount)
            .unwrap()
            <= offer.sell_token_end_amount,
        TakeOfferErrorCode::OfferExceedsSellLimit
    );

    let current_sell_token_amount = calculate_current_sell_amount(&offer).unwrap();

    let buy_token_1_amount = calculate_buy_amount(
        sell_token_amount,
        offer.buy_token_1.amount,
        current_sell_token_amount,
    )?;
    require!(
        ctx.accounts.offer_buy_token_1_account.amount >= buy_token_1_amount,
        TakeOfferErrorCode::InsufficientOfferBalance
    );

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_sell_token_account.to_account_info(),
                to: ctx.accounts.offer_sell_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        sell_token_amount,
    )?;

    let offer_id_bytes = &offer.offer_id.to_le_bytes();
    let seeds = &[
        b"offer_authority".as_ref(),
        offer_id_bytes,
        &[offer.authority_bump],
    ];
    let signer_seeds = &[&seeds[..]];
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.offer_buy_token_1_account.to_account_info(),
                to: ctx.accounts.user_buy_token_1_account.to_account_info(),
                authority: ctx.accounts.offer_token_authority.to_account_info(),
            },
            signer_seeds,
        ),
        buy_token_1_amount,
    )?;

    emit!(OfferTakenOne {
        offer_id: offer.offer_id,
        user: ctx.accounts.user.key(),
        sell_token_amount,
        buy_token_1_amount,
        remaining_sell_token_amount: offer
            .sell_token_end_amount
            .checked_sub(ctx.accounts.offer_sell_token_account.amount)
            .unwrap(),
    });

    Ok(())
}

/// Account structure for taking an offer with two buy tokens.
///
/// This struct defines the accounts required to accept an offer, exchanging sell tokens
/// for two buy tokens from the offer.
///
/// # Preconditions
/// - All Associated Token Accounts (ATAs) must be initialized prior to execution.
///   This includes `offer_sell_token_account`, `offer_buy_token_1_account`, `offer_buy_token_2_account`,
///   `user_sell_token_account`, `user_buy_token_1_account`, and `user_buy_token_2_account`.
#[derive(Accounts)]
pub struct TakeOfferTwo<'info> {
    /// The offer account being taken, providing offer details.
    #[account(
        constraint = offer.buy_token_2.mint != system_program::ID @ TakeOfferErrorCode::InvalidTakeOffer
    )]
    pub offer: Account<'info, Offer>,

    /// Offer's sell token ATA, receives the user's sell tokens.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_sell_token_account: Account<'info, TokenAccount>,

    /// Offer's buy token 1 ATA, sends buy token 1 to the user.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_1.mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_token_1_account: Account<'info, TokenAccount>,

    /// Offer's buy token 2 ATA, sends buy token 2 to the user.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_2.mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_token_2_account: Account<'info, TokenAccount>,

    /// User's sell token account, sends sell tokens to the offer.
    /// Ensures mint matches the offer's sell token mint.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = user,
        constraint = offer.sell_token_mint == user_sell_token_account.mint @ TakeOfferErrorCode::InvalidSellTokenMint
  )]
    pub user_sell_token_account: Account<'info, TokenAccount>,

    /// User's buy token 1 ATA, receives buy token 1 from the offer.
    /// Ensures mint matches the offer's buy token 1 mint.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_1.mint,
        associated_token::authority = user,
        constraint = offer.buy_token_1.mint == user_buy_token_1_account.mint @ TakeOfferErrorCode::InvalidBuyTokenMint
  )]
    pub user_buy_token_1_account: Account<'info, TokenAccount>,

    /// User's buy token 2 ATA, receives buy token 2 from the offer.
    /// Ensures mint matches the offer's buy token 2 mint.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_2.mint,
        associated_token::authority = user,
        constraint = offer.buy_token_2.mint == user_buy_token_2_account.mint @ TakeOfferErrorCode::InvalidBuyTokenMint
  )]
    pub user_buy_token_2_account: Account<'info, TokenAccount>,

    /// Derived PDA for token authority, controls offer token accounts.
    ///
    /// # Note
    /// This account is marked with `CHECK` as it's validated by the seed derivation.
    #[account(
        seeds = [b"offer_authority", offer.offer_id.to_le_bytes().as_ref()],
        bump
  )]
    pub offer_token_authority: AccountInfo<'info>,

    /// The user taking the offer, signs the transaction.
    #[account(mut)]
    pub user: Signer<'info>,

    /// SPL Token program for token operations.
    pub token_program: Program<'info, Token>,

    /// Solana System program, included for potential rent accounting.
    pub system_program: Program<'info, System>,
}

/// Takes an offer with two buy tokens.
///
/// Allows a user to exchange sell tokens for two buy tokens from the offer, transferring
/// tokens between accounts and emitting an `OfferTakenTwo` event with transaction details.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the offer take.
/// - `sell_token_amount`: Amount of sell tokens the user provides.
///
/// # Errors
/// - [`TakeOfferErrorCode::OfferExceedsSellLimit`] if the sell amount exceeds the offer's limit.
/// - [`TakeOfferErrorCode::InsufficientOfferBalance`] if the offer lacks sufficient buy tokens.
/// - [`TakeOfferErrorCode::CalculationOverflow`] if amount calculations overflow.
pub fn take_offer_two(ctx: Context<TakeOfferTwo>, sell_token_amount: u64) -> Result<()> {
    let offer = &ctx.accounts.offer;

    require!(
        ctx.accounts
            .offer_sell_token_account
            .amount
            .checked_add(sell_token_amount)
            .unwrap()
            <= offer.sell_token_end_amount,
        TakeOfferErrorCode::OfferExceedsSellLimit
    );

    let current_sell_token_amount = calculate_current_sell_amount(&offer).unwrap();

    let buy_token_1_amount = calculate_buy_amount(
        sell_token_amount,
        offer.buy_token_1.amount,
        current_sell_token_amount,
    )?;
    let buy_token_2_amount = calculate_buy_amount(
        sell_token_amount,
        offer.buy_token_2.amount,
        current_sell_token_amount,
    )?;
    require!(
        ctx.accounts.offer_buy_token_1_account.amount >= buy_token_1_amount,
        TakeOfferErrorCode::InsufficientOfferBalance
    );
    require!(
        ctx.accounts.offer_buy_token_2_account.amount >= buy_token_2_amount,
        TakeOfferErrorCode::InsufficientOfferBalance
    );

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_sell_token_account.to_account_info(),
                to: ctx.accounts.offer_sell_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        sell_token_amount,
    )?;

    let offer_id_bytes = &offer.offer_id.to_le_bytes();
    let seeds = &[
        b"offer_authority".as_ref(),
        offer_id_bytes,
        &[offer.authority_bump],
    ];
    let signer_seeds = &[&seeds[..]];
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.offer_buy_token_1_account.to_account_info(),
                to: ctx.accounts.user_buy_token_1_account.to_account_info(),
                authority: ctx.accounts.offer_token_authority.to_account_info(),
            },
            signer_seeds,
        ),
        buy_token_1_amount,
    )?;
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.offer_buy_token_2_account.to_account_info(),
                to: ctx.accounts.user_buy_token_2_account.to_account_info(),
                authority: ctx.accounts.offer_token_authority.to_account_info(),
            },
            signer_seeds,
        ),
        buy_token_2_amount,
    )?;

    emit!(OfferTakenTwo {
        offer_id: offer.offer_id,
        user: ctx.accounts.user.key(),
        sell_token_amount,
        buy_token_1_amount,
        buy_token_2_amount,
        remaining_sell_token_amount: offer
            .sell_token_end_amount
            .checked_sub(ctx.accounts.offer_sell_token_account.amount)
            .unwrap(),
    });

    Ok(())
}

/// Calculates the proportional buy token amount based on sell token input.
///
/// # Arguments
/// - `sell_token_amount`: Amount of sell tokens provided by the user.
/// - `offer_buy_token_amount`: Amount of buy tokens in the offer, used for price calculation.
/// - `offer_sell_token_amount`: Current amount of sell tokens in the offer, used for price calculation.
///
/// # Returns
/// The calculated amount of buy tokens to transfer, or an error if calculations fail.
///
/// # Errors
/// - [`TakeOfferErrorCode::InvalidSellTokenMint`] if `sell_token_total_amount` is zero.
/// - [`TakeOfferErrorCode::CalculationOverflow`] if multiplication or division overflows.
/// - [`TakeOfferErrorCode::ZeroBuyTokenAmount`] if the result is zero.
fn calculate_buy_amount(
    user_sell_token_amount: u64,
    offer_buy_token_amount: u64,
    offer_sell_token_amount: u64,
) -> Result<u64> {
    if offer_sell_token_amount == 0 {
        return Err(error!(TakeOfferErrorCode::InvalidSellTokenMint).into());
    }
    let result = (user_sell_token_amount as u128)
        .checked_mul(offer_buy_token_amount as u128)
        .ok_or(TakeOfferErrorCode::CalculationOverflow)?
        .checked_div(offer_sell_token_amount as u128)
        .ok_or(TakeOfferErrorCode::CalculationOverflow)?;
    if result > u64::MAX as u128 {
        return Err(error!(TakeOfferErrorCode::CalculationOverflow));
    }
    if result == 0 {
        return Err(error!(TakeOfferErrorCode::ZeroBuyTokenAmount));
    }
    Ok(result as u64)
}

/// Error codes for offer taking operations.
#[error_code]
pub enum TakeOfferErrorCode {
    /// Triggered when the offer lacks sufficient buy tokens to fulfill the take.
    #[msg("Insufficient tokens remaining in the offer.")]
    InsufficientOfferBalance,

    /// Triggered when the user's sell token mint doesn't match the offer's.
    #[msg("The sell token mint does not match the offer.")]
    InvalidSellTokenMint,

    /// Triggered when the user's buy token mint doesn't match the offer's.
    #[msg("The buy token mint does not match the offer.")]
    InvalidBuyTokenMint,

    /// Triggered when the sell amount exceeds the offer's total sell limit.
    #[msg("The offer would exceed its total sell token limit.")]
    OfferExceedsSellLimit,

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
    #[msg("Current time must be within the offer's start and end time range")]
    InvalidCurrentTime,
}
