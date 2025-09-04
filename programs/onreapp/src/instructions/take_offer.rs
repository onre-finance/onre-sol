use crate::state::Offer;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

/// Event emitted when an offer with one buy token is taken.
#[event]
pub struct OfferTakenOne {
    pub offer_id: u64,
    pub user: Pubkey,
    pub sell_token_amount: u64,
    pub buy_token_1_amount: u64,
    pub remaining_buy_token_amount: u64,
}

/// Event emitted when an offer with two buy tokens is taken.
#[event]
pub struct OfferTakenTwo {
    pub offer_id: u64,
    pub user: Pubkey,
    pub sell_token_amount: u64,
    pub buy_token_1_amount: u64,
    pub buy_token_2_amount: u64,
    pub remaining_buy_token_1_amount: u64,
    pub remaining_buy_token_2_amount: u64,
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
        constraint = offer.buy_token_2.mint == Pubkey::default() @ TakeOfferErrorCode::InvalidTakeOffer
    )]
    pub offer: Account<'info, Offer>,

    /// Offer's sell token ATA, receives the user's sell tokens.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = offer_token_authority,
    )]
    pub offer_sell_token_account: Box<Account<'info, TokenAccount>>,

    /// Offer's buy token 1 ATA, sends buy tokens to the user.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_1.mint,
        associated_token::authority = offer_token_authority,
    )]
    pub offer_buy_token_1_account: Box<Account<'info, TokenAccount>>,

    /// User's sell token ATA, sends sell tokens to the offer.
    /// Ensures mint matches the offer's sell token mint.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = user,
        constraint = offer.sell_token_mint == user_sell_token_account.mint @ TakeOfferErrorCode::InvalidSellTokenMint
    )]
    pub user_sell_token_account: Box<Account<'info, TokenAccount>>,

    /// User's buy token 1 ATA, receives buy tokens from the offer.
    /// Ensures mint matches the offer's buy token 1 mint.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_1.mint,
        associated_token::authority = user,
        constraint = offer.buy_token_1.mint == user_buy_token_1_account.mint @ TakeOfferErrorCode::InvalidBuyTokenMint
    )]
    pub user_buy_token_1_account: Box<Account<'info, TokenAccount>>,

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
    pub user: Signer<'info>,

    /// SPL Token program for token operations.
    pub token_program: Program<'info, Token>,

    /// Solana System program, included for potential rent accounting.
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
/// - [`TakeOfferErrorCode::InvalidCurrentTime`] if the current time is outside the offer's active period.
fn calculate_current_sell_amount(
    offer: &Offer
) -> Result<u64> {
    let current_time = Clock::get()?.unix_timestamp as u64;
    
    require!(
        current_time >= offer.offer_start_time && current_time < offer.offer_end_time,
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
        .checked_add(sell_token_amount_per_interval.checked_mul(current_interval + 1).unwrap())
        .unwrap();

    Ok(sell_token_current_amount)
}

/// Takes an offer with one buy token.
///
/// Allows a user to exchange their sell tokens for buy tokens from the offer.
/// The amount of buy token received depends on the `sell_token_amount` provided by the user
/// and the current price determined by the offer's dynamic pricing parameters
/// (`sell_token_start_amount`, `sell_token_end_amount`, `offer_start_time`, `offer_end_time`, `price_fix_duration`).
/// Transfers tokens between accounts and emits an `OfferTakenOne` event.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the offer take.
/// - `sell_token_amount`: Amount of sell tokens the user provides to exchange for buy tokens.
///
/// # Errors
/// - [`TakeOfferErrorCode::InvalidCurrentTime`] if the offer is not active.
/// - [`TakeOfferErrorCode::InsufficientOfferTokenOneBalance`] if the offer lacks sufficient buy tokens to fulfill the exchange at the current price.
/// - [`TakeOfferErrorCode::CalculationOverflow`] if intermediate amount calculations overflow.
/// - [`TakeOfferErrorCode::ZeroBuyTokenAmount`] if the calculated buy token amount to be received is zero.
pub fn take_offer_one(ctx: Context<TakeOfferOne>, sell_token_amount: u64) -> Result<()> {
    let offer = &ctx.accounts.offer;

    let current_sell_token_amount = calculate_current_sell_amount(&offer).unwrap();
    msg!("Calculated current sell token amount: {}", current_sell_token_amount);

    let buy_token_1_amount = calculate_buy_amount(
        sell_token_amount,
        offer.buy_token_1.amount,
        current_sell_token_amount,
    )?;
    
    msg!("Calculated buy token 1 amount: {}", buy_token_1_amount);
    require!(
        ctx.accounts.offer_buy_token_1_account.amount >= buy_token_1_amount,
        TakeOfferErrorCode::InsufficientOfferTokenOneBalance
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
    msg!("Transferring {} sell tokens from user to offer", sell_token_amount);

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
    msg!("Transferring {} buy tokens 1 from offer to user", buy_token_1_amount);
    emit!(OfferTakenOne {
        offer_id: offer.offer_id,
        user: ctx.accounts.user.key(),
        sell_token_amount,
        buy_token_1_amount,
        remaining_buy_token_amount: ctx.accounts.offer_buy_token_1_account.amount - buy_token_1_amount
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
        constraint = offer.buy_token_2.mint != Pubkey::default() @ TakeOfferErrorCode::InvalidTakeOffer
    )]
    pub offer: Account<'info, Offer>,

    /// Offer's sell token ATA, receives the user's sell tokens.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_sell_token_account: Box<Account<'info, TokenAccount>>,

    /// Offer's buy token 1 ATA, sends buy token 1 to the user.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_1.mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_token_1_account: Box<Account<'info, TokenAccount>>,

    /// Offer's buy token 2 ATA, sends buy token 2 to the user.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_2.mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_token_2_account: Box<Account<'info, TokenAccount>>,

    /// User's sell token account, sends sell tokens to the offer.
    /// Ensures mint matches the offer's sell token mint.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = user,
        constraint = offer.sell_token_mint == user_sell_token_account.mint @ TakeOfferErrorCode::InvalidSellTokenMint
  )]
    pub user_sell_token_account: Box<Account<'info, TokenAccount>>,

    /// User's buy token 1 ATA, receives buy token 1 from the offer.
    /// Ensures mint matches the offer's buy token 1 mint.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_1.mint,
        associated_token::authority = user,
        constraint = offer.buy_token_1.mint == user_buy_token_1_account.mint @ TakeOfferErrorCode::InvalidBuyTokenMint
  )]
    pub user_buy_token_1_account: Box<Account<'info, TokenAccount>>,

    /// User's buy token 2 ATA, receives buy token 2 from the offer.
    /// Ensures mint matches the offer's buy token 2 mint.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_2.mint,
        associated_token::authority = user,
        constraint = offer.buy_token_2.mint == user_buy_token_2_account.mint @ TakeOfferErrorCode::InvalidBuyTokenMint
  )]
    pub user_buy_token_2_account: Box<Account<'info, TokenAccount>>,

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
    pub user: Signer<'info>,

    /// SPL Token program for token operations.
    pub token_program: Program<'info, Token>,

    /// Solana System program, included for potential rent accounting.
    pub system_program: Program<'info, System>,
}

/// Takes an offer with two buy tokens, considering the dynamic pricing model.
///
/// Allows a user to exchange their sell tokens for two types of buy tokens from the offer.
/// The amount of each buy token received depends on the `sell_token_amount` provided by the user
/// and the current price determined by the offer's dynamic pricing parameters
/// (`sell_token_start_amount`, `sell_token_end_amount`, `offer_start_time`, `offer_end_time`, `price_fix_duration`).
/// Transfers tokens between accounts and emits an `OfferTakenTwo` event.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the offer take.
/// - `sell_token_amount`: Amount of sell tokens the user provides to exchange for buy tokens.
///
/// # Errors
/// - [`TakeOfferErrorCode::InvalidCurrentTime`] if the offer is not active.
/// - [`TakeOfferErrorCode::InsufficientOfferTokenOneBalance`] if the offer lacks sufficient quantity of buy token 1 to fulfill the exchange at the current price.
/// - [`TakeOfferErrorCode::InsufficientOfferTokenTwoBalance`] if the offer lacks sufficient quantity of buy token 2 to fulfill the exchange at the current price.
/// - [`TakeOfferErrorCode::CalculationOverflow`] if intermediate amount calculations overflow.
/// - [`TakeOfferErrorCode::ZeroBuyTokenAmount`] if the calculated amount for either buy token to be received is zero.
pub fn take_offer_two(ctx: Context<TakeOfferTwo>, sell_token_amount: u64) -> Result<()> {
    let offer = &ctx.accounts.offer;

    let current_sell_token_amount = calculate_current_sell_amount(&offer).unwrap();
    msg!("Calculated current sell token amount: {}", current_sell_token_amount);

    let buy_token_1_amount = calculate_buy_amount(
        sell_token_amount,
        offer.buy_token_1.amount,
        current_sell_token_amount,
    )?;
    msg!("Calculated buy token 1 amount: {}", buy_token_1_amount);
    require!(
        ctx.accounts.offer_buy_token_1_account.amount >= buy_token_1_amount,
        TakeOfferErrorCode::InsufficientOfferTokenOneBalance
    );

    let buy_token_2_amount = calculate_buy_amount(
        sell_token_amount,
        offer.buy_token_2.amount,
        current_sell_token_amount,
    )?;
    msg!("Calculated buy token 2 amount: {}", buy_token_2_amount);
    require!(
        ctx.accounts.offer_buy_token_2_account.amount >= buy_token_2_amount,
        TakeOfferErrorCode::InsufficientOfferTokenTwoBalance
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
    msg!("Transferring {} sell tokens from user to offer", sell_token_amount);

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
    msg!("Transferring {} buy tokens 1 from offer to user", buy_token_1_amount);
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
    msg!("Transferring {} buy tokens 2 from offer to user", buy_token_2_amount);

    emit!(OfferTakenTwo {
        offer_id: offer.offer_id,
        user: ctx.accounts.user.key(),
        sell_token_amount,
        buy_token_1_amount,
        buy_token_2_amount,
        remaining_buy_token_1_amount: ctx.accounts.offer_buy_token_1_account.amount - buy_token_1_amount,
        remaining_buy_token_2_amount: ctx.accounts.offer_buy_token_2_account.amount - buy_token_2_amount,
    });

    Ok(())
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
/// - [`TakeOfferErrorCode::InvalidSellTokenMint`] if `offer_sell_token_amount` (the denominator) is zero.
/// - [`TakeOfferErrorCode::CalculationOverflow`] if multiplication or division results in overflow.
/// - [`TakeOfferErrorCode::ZeroBuyTokenAmount`] if the calculated buy token amount for the user is zero.
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
    #[msg("Insufficient tokens remaining in the offer for token 1.")]
    InsufficientOfferTokenOneBalance,

    #[msg("Insufficient tokens remaining in the offer for token 2.")]
    InsufficientOfferTokenTwoBalance,

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
