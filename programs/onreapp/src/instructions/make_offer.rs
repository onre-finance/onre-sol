use crate::contexts::MakeOfferContext;
use crate::state::{Offer, OfferToken, State};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

/// Event emitted when an offer with one buy token is created.
#[event]
pub struct OfferMadeOne {
    pub offer_id: u64,
    pub boss: Pubkey,
    pub buy_token_total_amount: u64,
    pub sell_token_start_amount: u64,
    pub sell_token_end_amount: u64,
    pub offer_start_time: u64,
    pub offer_end_time: u64,
    pub price_fix_duration: u64,
}

/// Event emitted when an offer with two buy tokens is created.
#[event]
pub struct OfferMadeTwo {
    pub offer_id: u64,
    pub boss: Pubkey,
    pub buy_token_1_total_amount: u64,
    pub buy_token_2_total_amount: u64,
    pub sell_token_start_amount: u64,
    pub sell_token_end_amount: u64,
    pub offer_start_time: u64,
    pub offer_end_time: u64,
    pub price_fix_duration: u64,
}

/// Account structure for creating an offer with one buy token.
///
/// This struct defines the accounts required to initialize an offer where the boss provides
/// a single buy token in exchange for a sell token. The price of the sell token can change
/// dynamically over the offer's duration.
///
/// # Preconditions
/// - All Associated Token Accounts (ATAs) must be initialized prior to execution.
///   This includes `offer_sell_token_account`, `offer_buy_token_1_account`, and `boss_buy_token_1_account`.
#[derive(Accounts)]
#[instruction(offer_id: u64)]
pub struct MakeOfferOne<'info> {
    /// The offer account to be initialized, with rent paid by `boss`.
    ///
    /// # Note
    /// - Space is allocated as `8 + Offer::INIT_SPACE` bytes, where 8 bytes are for the discriminator.
    /// - Seeded with `"offer"` and `offer_id` for PDA derivation.
    #[account(
        init,
        payer = boss,
        space = 8 + Offer::INIT_SPACE,
        seeds = [b"offer", offer_id.to_le_bytes().as_ref()],
        bump
  )]
    pub offer: Account<'info, Offer>,

    /// Offer's sell token ATA, must exist prior to execution, controlled by `offer_token_authority`.
    ///
    /// # Note
    /// Included for future sell token transfers when the offer is taken.
    #[account(
        associated_token::mint = sell_token_mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_sell_token_account: Account<'info, TokenAccount>,

    /// Offer's buy token 1 ATA, must exist prior to execution, controlled by `offer_token_authority`.
    #[account(
        mut,
        associated_token::mint = buy_token_1_mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_token_1_account: Account<'info, TokenAccount>,

    /// Derived PDA for token authority, does not store data.
    ///
    /// # Note
    /// This account is marked with `CHECK` as it's validated by the seed derivation.
    #[account(
        seeds = [b"offer_authority", offer_id.to_le_bytes().as_ref()],
        bump
  )]
    pub offer_token_authority: AccountInfo<'info>,

    /// Boss's buy token 1 ATA, must exist prior to execution, owned by `boss`.
    #[account(
        mut,
        associated_token::mint = buy_token_1_mint,
        associated_token::authority = boss,
  )]
    pub boss_buy_token_1_account: Box<Account<'info, TokenAccount>>,

    /// Mint of the sell token for the offer.
    pub sell_token_mint: Box<Account<'info, Mint>>,

    /// Mint of the buy token 1 for the offer.
    pub buy_token_1_mint: Box<Account<'info, Mint>>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The signer funding and authorizing the offer creation, typically the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// SPL Token program for token operations.
    pub token_program: Program<'info, Token>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Trait implementation for `MakeOfferOne` to satisfy `MakeOfferContext`.
impl<'info> MakeOfferContext<'info> for MakeOfferOne<'info> {
    fn token_program(&self) -> &Program<'info, Token> {
        &self.token_program
    }

    fn boss(&self) -> &AccountInfo<'info> {
        &self.boss
    }

    fn offer(&self) -> &Account<'info, Offer> {
        &self.offer
    }
}

/// Trait implementation for `MakeOfferTwo` to satisfy `MakeOfferContext`.
impl<'info> MakeOfferContext<'info> for MakeOfferTwo<'info> {
    fn token_program(&self) -> &Program<'info, Token> {
        &self.token_program
    }

    fn boss(&self) -> &AccountInfo<'info> {
        &self.boss
    }

    fn offer(&self) -> &Account<'info, Offer> {
        &self.offer
    }
}

/// Account structure for creating an offer with two buy tokens.
///
/// This struct defines the accounts required to initialize an offer where the boss provides
/// two buy tokens in exchange for a sell token. The price of the sell token can change
/// dynamically over the offer's duration.
///
/// # Preconditions
/// - All Associated Token Accounts (ATAs) must be initialized prior to execution.
///   This includes `offer_sell_token_account`, `offer_buy_token_1_account`, `offer_buy_token_2_account`,
///   `boss_buy_token_1_account`, and `boss_buy_token_2_account`.
#[derive(Accounts)]
#[instruction(offer_id: u64)]
pub struct MakeOfferTwo<'info> {
    /// The offer account to be initialized, with rent paid by `boss`.
    ///
    /// # Note
    /// - Space is allocated as `8 + Offer::INIT_SPACE` bytes, where 8 bytes are for the discriminator.
    /// - Seeded with `"offer"` and `offer_id` for PDA derivation.
    #[account(
        init,
        payer = boss,
        space = 8 + Offer::INIT_SPACE,
        seeds = [b"offer", offer_id.to_le_bytes().as_ref()],
        bump
  )]
    pub offer: Account<'info, Offer>,

    /// Offer's sell token ATA, must exist prior to execution, controlled by `offer_token_authority`.
    ///
    /// # Note
    /// Included for future sell token transfers when the offer is taken.
    #[account(
        associated_token::mint = sell_token_mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_sell_token_account: Account<'info, TokenAccount>,

    /// Offer's buy token 1 ATA, must exist prior to execution, controlled by `offer_token_authority`.
    #[account(
        mut,
        associated_token::mint = buy_token_1_mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_token_1_account: Account<'info, TokenAccount>,

    /// Offer's buy token 2 ATA, must exist prior to execution, controlled by `offer_token_authority`.
    #[account(
        mut,
        associated_token::mint = buy_token_2_mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_token_2_account: Account<'info, TokenAccount>,

    /// Derived PDA for token authority, does not store data.
    ///
    /// # Note
    /// This account is marked with `CHECK` as it's validated by the seed derivation.
    #[account(
        seeds = [b"offer_authority", offer_id.to_le_bytes().as_ref()],
        bump
  )]
    pub offer_token_authority: AccountInfo<'info>,

    /// Boss's buy token 1 ATA, must exist prior to execution, owned by `boss`.
    #[account(
        mut,
        associated_token::mint = buy_token_1_mint,
        associated_token::authority = boss,
  )]
    pub boss_buy_token_1_account: Box<Account<'info, TokenAccount>>,

    /// Boss's buy token 2 ATA, must exist prior to execution, owned by `boss`.
    #[account(
        mut,
        associated_token::mint = buy_token_2_mint,
        associated_token::authority = boss,
  )]
    pub boss_buy_token_2_account: Box<Account<'info, TokenAccount>>,

    /// Mint of the sell token for the offer.
    pub sell_token_mint: Box<Account<'info, Mint>>,

    /// Mint of the buy token 1 for the offer.
    pub buy_token_1_mint: Box<Account<'info, Mint>>,

    /// Mint of the buy token 2 for the offer.
    pub buy_token_2_mint: Box<Account<'info, Mint>>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The signer funding and authorizing the offer creation, typically the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// SPL Token program for token operations.
    pub token_program: Program<'info, Token>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Creates an offer with one buy token.
///
/// Initializes an offer where the boss provides one buy token. The amount of sell token
/// required in exchange varies over time, determined by `sell_token_start_amount`,
/// `sell_token_end_amount`, `offer_start_time`, `offer_end_time`, and `price_fix_duration`.
/// Transfers the specified `buy_token_total_amount` from the boss to the offer's account
/// and emits an `OfferMadeOne` event.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the offer.
/// - `offer_id`: Unique identifier for the offer, used in PDA derivation.
/// - `buy_token_total_amount`: Total amount of the buy token to be offered.
/// - `sell_token_start_amount`: The amount of sell token expected in exchange at the beginning of the offer.
/// - `sell_token_end_amount`: The amount of sell token expected in exchange at the end of the offer.
/// - `offer_start_time`: Unix timestamp for when the offer becomes active.
/// - `offer_end_time`: Unix timestamp for when the offer expires.
/// - `price_fix_duration`: Duration in seconds for each price interval, in which the price of the buy token is fixed. The amount of sell tokens
///   used to calculate the current price of the buy token interpolates linearly between `sell_token_start_amount` and `sell_token_end_amount` over
///   the total number of intervals.
///
/// # Errors
/// - [`MakeOfferErrorCode::InsufficientBalance`] if the boss lacks sufficient `buy_token_total_amount`.
/// - [`MakeOfferErrorCode::InvalidAmount`] if `buy_token_total_amount`, `sell_token_start_amount`,
///   or `sell_token_end_amount` is zero. Also if `sell_token_start_amount` > `sell_token_end_amount`.
/// - [`MakeOfferErrorCode::InvalidOfferTime`] if `offer_start_time` is not less than `offer_end_time`.
/// - [`MakeOfferErrorCode::InvalidPriceFixDuration`] if `price_fix_duration` is zero or if the total
///   offer duration is less than `price_fix_duration`.
pub fn make_offer_one(
    ctx: Context<MakeOfferOne>,
    offer_id: u64,
    buy_token_total_amount: u64,
    sell_token_start_amount: u64,
    sell_token_end_amount: u64,
    offer_start_time: u64,
    offer_end_time: u64,
    price_fix_duration: u64,
) -> Result<()> {
    validate_non_zero_token_amounts(&[buy_token_total_amount, sell_token_start_amount, sell_token_end_amount])?;
    validate_dynamic_price_params(sell_token_start_amount, sell_token_end_amount, offer_start_time, offer_end_time, price_fix_duration)?;

    require!(
        ctx.accounts.boss_buy_token_1_account.amount >= buy_token_total_amount,
        MakeOfferErrorCode::InsufficientBalance
    );

    let offer = &mut ctx.accounts.offer;
    offer.offer_id = offer_id;
    offer.sell_token_mint = ctx.accounts.sell_token_mint.key();
    offer.sell_token_start_amount = sell_token_start_amount;
    offer.sell_token_end_amount = sell_token_end_amount;
    offer.price_fix_duration = price_fix_duration;
    offer.offer_start_time = offer_start_time;
    offer.offer_end_time = offer_end_time;

    offer.buy_token_1 = OfferToken { mint: ctx.accounts.buy_token_1_mint.key(), amount: buy_token_total_amount };
    offer.buy_token_2 = OfferToken { mint: Pubkey::default(), amount: 0 };
    offer.authority_bump = ctx.bumps.offer_token_authority;

    transfer_token(
        &ctx,
        &ctx.accounts.boss_buy_token_1_account,
        &ctx.accounts.offer_buy_token_1_account,
        buy_token_total_amount,
    )?;
    msg!("Transferring {} buy tokens 1 from boss to offer", buy_token_total_amount);
    msg!("Offer created with buy_token_total_amount: {}, sell_token_start_amount: {}, sell_token_end_amount: {} and price_fix_duration: {}", 
        buy_token_total_amount, 
        sell_token_start_amount, 
        sell_token_end_amount, 
        price_fix_duration
    );

    emit!(OfferMadeOne {
        offer_id,
        boss: ctx.accounts.boss.key(),
        buy_token_total_amount,
        sell_token_start_amount,
        sell_token_end_amount,
        offer_start_time,
        offer_end_time,
        price_fix_duration,
    });

    Ok(())
}

/// Creates an offer with two buy tokens.
///
/// Initializes an offer where the boss provides two buy tokens in exchange for a sell token. The amount of sell token
/// required in exchange varies over time, determined by `sell_token_start_amount`,
/// `sell_token_end_amount`, `offer_start_time`, `offer_end_time`, and `price_fix_duration`.
/// Transfers the specified amounts of buy tokens from the boss to the offer's accounts
/// and emits an `OfferMadeTwo` event.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the offer.
/// - `offer_id`: Unique identifier for the offer, used in PDA derivation.
/// - `buy_token_1_total_amount`: Total amount of the first buy token to be offered.
/// - `buy_token_2_total_amount`: Total amount of the second buy token to be offered.
/// - `sell_token_start_amount`: The amount of sell token expected in exchange at the beginning of the offer.
/// - `sell_token_end_amount`: The amount of sell token expected in exchange at the end of the offer.
/// - `offer_start_time`: Unix timestamp for when the offer becomes active.
/// - `offer_end_time`: Unix timestamp for when the offer expires.
/// - `price_fix_duration`: Duration in seconds for each price interval, in which the prices of the buy tokens 
///   are fixed. The amount of sell tokens expected in exchange interpolates linearly between `sell_token_start_amount` 
///   and `sell_token_end_amount` over the total number of intervals.
///
/// # Errors
/// - [`MakeOfferErrorCode::InsufficientBalance`] if the boss lacks sufficient amounts for
///   `buy_token_1_total_amount` or `buy_token_2_total_amount`.
/// - [`MakeOfferErrorCode::InvalidAmount`] if any buy token amount, `sell_token_start_amount`,
///   or `sell_token_end_amount` is zero. Also if `sell_token_start_amount` > `sell_token_end_amount`.
/// - [`MakeOfferErrorCode::InvalidOfferTime`] if `offer_start_time` is not less than `offer_end_time`.
/// - [`MakeOfferErrorCode::InvalidPriceFixDuration`] if `price_fix_duration` is zero or if the total
///   offer duration is less than `price_fix_duration`.
pub fn make_offer_two(
    ctx: Context<MakeOfferTwo>,
    offer_id: u64,
    buy_token_1_total_amount: u64,
    buy_token_2_total_amount: u64,
    sell_token_start_amount: u64,
    sell_token_end_amount: u64,
    offer_start_time: u64,
    offer_end_time: u64,
    price_fix_duration: u64,
) -> Result<()> {
    validate_non_zero_token_amounts(&[buy_token_1_total_amount, buy_token_2_total_amount, sell_token_start_amount, sell_token_end_amount])?;
    validate_dynamic_price_params(sell_token_start_amount, sell_token_end_amount, offer_start_time, offer_end_time, price_fix_duration)?;

    require!(
        ctx.accounts.boss_buy_token_1_account.amount >= buy_token_1_total_amount,
        MakeOfferErrorCode::InsufficientBalance
    );
    require!(
        ctx.accounts.boss_buy_token_2_account.amount >= buy_token_2_total_amount,
        MakeOfferErrorCode::InsufficientBalance
    );

    let offer = &mut ctx.accounts.offer;
    offer.offer_id = offer_id;
    offer.sell_token_mint = ctx.accounts.sell_token_mint.key();
    offer.buy_token_1 = OfferToken { mint: ctx.accounts.buy_token_1_mint.key(), amount: buy_token_1_total_amount };
    offer.buy_token_2 = OfferToken { mint: ctx.accounts.buy_token_2_mint.key(), amount: buy_token_2_total_amount };
    offer.sell_token_start_amount = sell_token_start_amount;
    offer.sell_token_end_amount = sell_token_end_amount;
    offer.price_fix_duration = price_fix_duration;
    offer.offer_start_time = offer_start_time;
    offer.offer_end_time = offer_end_time;
    offer.authority_bump = ctx.bumps.offer_token_authority;

    transfer_token(
        &ctx,
        &ctx.accounts.boss_buy_token_1_account,
        &ctx.accounts.offer_buy_token_1_account,
        buy_token_1_total_amount,
    )?;
    msg!("Transferring {} buy tokens 1 from boss to offer", buy_token_1_total_amount);

    transfer_token(
        &ctx,
        &ctx.accounts.boss_buy_token_2_account,
        &ctx.accounts.offer_buy_token_2_account,
        buy_token_2_total_amount,
    )?;
    msg!("Transferring {} buy tokens 2 from boss to offer", buy_token_2_total_amount);
    msg!("Offer created with buy_token_1_total_amount: {}, buy_token_2_total_amount: {}, sell_token_start_amount: {}, sell_token_end_amount: {} and price_fix_duration: {}", 
        buy_token_1_total_amount, 
        buy_token_2_total_amount, 
        sell_token_start_amount, 
        sell_token_end_amount, 
        price_fix_duration
    );

    emit!(OfferMadeTwo {
        offer_id,
        boss: ctx.accounts.boss.key(),
        buy_token_1_total_amount,
        buy_token_2_total_amount,
        sell_token_start_amount,
        sell_token_end_amount,
        offer_start_time,
        offer_end_time,
        price_fix_duration,
    });

    Ok(())
}

/// Transfers tokens from a source to a destination account.
///
/// # Arguments
/// - `ctx`: Context containing program and account information.
/// - `from`: Source token account to transfer from.
/// - `to`: Destination token account to transfer to.
/// - `amount`: Amount of tokens to transfer.
///
/// # Errors
/// - [`MakeOfferErrorCode::InvalidAmount`] if the amount is zero, or sell token amounts are invalid.
/// - Fails if the boss lacks sufficient authority or balance (handled by SPL Token program).
fn transfer_token<'info, T: MakeOfferContext<'info> + anchor_lang::Bumps>(
    ctx: &Context<T>,
    from: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, MakeOfferErrorCode::InvalidAmount);
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program().to_account_info(),
        Transfer {
            from: from.to_account_info(),
            to: to.to_account_info(),
            authority: ctx.accounts.boss().to_account_info(),
        },
    );
    token::transfer(cpi_ctx, amount)?;
    Ok(())
}

fn validate_non_zero_token_amounts(token_amounts: &[u64]) -> Result<()> {
    require!(
        token_amounts.iter().all(|&x| x > 0),
        MakeOfferErrorCode::InvalidAmount
    );
    Ok(())
}

fn validate_dynamic_price_params(sell_token_start_amount: u64, sell_token_end_amount: u64, offer_start_time: u64, offer_end_time: u64, price_fix_duration: u64) -> Result<()> {
    require!(
        sell_token_start_amount <= sell_token_end_amount,
        MakeOfferErrorCode::InvalidAmount
    );
    require!(
        offer_start_time < offer_end_time,
        MakeOfferErrorCode::InvalidOfferTime
    );
    require!(
        price_fix_duration > 0,
        MakeOfferErrorCode::InvalidPriceFixDuration
    );
    require!(
        (offer_end_time - offer_start_time) >= price_fix_duration,
        MakeOfferErrorCode::InvalidPriceFixDuration
    );
    require!(
        (offer_end_time - offer_start_time) % price_fix_duration == 0,
        MakeOfferErrorCode::InvalidOfferTime
    );
    Ok(())
}

/// Error codes for offer creation operations.
#[error_code]
pub enum MakeOfferErrorCode {
    /// Triggered when the boss's token account doesn't have sufficient balance for the transfer.
    #[msg("Insufficient token balance in boss account")]
    InsufficientBalance,

    /// Triggered when the token transfer amount is zero, or sell token amounts are invalid.
    #[msg("Token transfer amount must be greater than zero. Sell token start amount must be > 0, end amount must be > 0, and start <= end.")]
    InvalidAmount,

    #[msg("Token offer end time must be greater than start time and end time - start time must be divisible by price fix duration")]
    InvalidOfferTime,

    /// Triggered when the price fix duration is invalid.
    #[msg("Price fix duration must be greater than zero and less than or equal to the total offer duration")]
    InvalidPriceFixDuration,
}
