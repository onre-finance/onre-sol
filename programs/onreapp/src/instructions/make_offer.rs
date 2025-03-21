use crate::contexts::MakeOfferContext;
use crate::state::{Offer, State};
use anchor_lang::prelude::*; // Includes `emit!` and `#[event]`
use anchor_lang::solana_program::system_program;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

/// Event emitted when an offer with one buy token is created.
#[event]
pub struct OfferMadeOne {
    pub offer_id: u64,
    pub boss: Pubkey,
    pub buy_token_1_total_amount: u64,
    pub sell_token_total_amount: u64,
}

/// Event emitted when an offer with two buy tokens is created.
#[event]
pub struct OfferMadeTwo {
    pub offer_id: u64,
    pub boss: Pubkey,
    pub buy_token_1_total_amount: u64,
    pub buy_token_2_total_amount: u64,
    pub sell_token_total_amount: u64,
}

/// Account structure for creating an offer with one buy token.
///
/// This struct defines the accounts required to initialize an offer where the boss provides
/// a single buy token in exchange for a sell token.
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
        associated_token::mint = buy_token_1_mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_token_1_account: Account<'info, TokenAccount>,

    /// Derived PDA for token authority, does not store data.
    ///
    /// # Note
    /// This account is marked with `CHECK` as it’s validated by the seed derivation.
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
/// two buy tokens in exchange for a sell token.
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
        associated_token::mint = buy_token_1_mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_token_1_account: Account<'info, TokenAccount>,

    /// Offer's buy token 2 ATA, must exist prior to execution, controlled by `offer_token_authority`.
    #[account(
        associated_token::mint = buy_token_2_mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_token_2_account: Account<'info, TokenAccount>,

    /// Derived PDA for token authority, does not store data.
    ///
    /// # Note
    /// This account is marked with `CHECK` as it’s validated by the seed derivation.
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

/// Creates an offer with two buy tokens.
///
/// Initializes an offer where the boss provides two buy tokens in exchange for a sell token.
/// Transfers the specified amounts of buy tokens from the boss to the offer’s accounts and emits
/// an `OfferMadeTwo` event for traceability.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the offer.
/// - `offer_id`: Unique identifier for the offer, used in PDA derivation.
/// - `buy_token_1_total_amount`: Total amount of buy token 1 to be offered.
/// - `buy_token_2_total_amount`: Total amount of buy token 2 to be offered.
/// - `sell_token_total_amount`: Total amount of sell token expected in exchange.
///
/// # Errors
/// - [`MakeOfferErrorCode::InsufficientBalance`] if the boss lacks sufficient buy token amounts.
/// - [`MakeOfferErrorCode::InvalidAmount`] if any transfer amount is zero.
pub fn make_offer_two(
    ctx: Context<MakeOfferTwo>,
    offer_id: u64,
    buy_token_1_total_amount: u64,
    buy_token_2_total_amount: u64,
    sell_token_total_amount: u64,
) -> Result<()> {
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
    offer.buy_token_mint_1 = ctx.accounts.buy_token_1_mint.key();
    offer.buy_token_mint_2 = ctx.accounts.buy_token_2_mint.key();
    offer.buy_token_1_total_amount = buy_token_1_total_amount;
    offer.buy_token_2_total_amount = buy_token_2_total_amount;
    offer.sell_token_total_amount = sell_token_total_amount;
    offer.authority_bump = ctx.bumps.offer_token_authority;

    transfer_token(
        &ctx,
        &ctx.accounts.boss_buy_token_1_account,
        &ctx.accounts.offer_buy_token_1_account,
        buy_token_1_total_amount,
    )?;

    transfer_token(
        &ctx,
        &ctx.accounts.boss_buy_token_2_account,
        &ctx.accounts.offer_buy_token_2_account,
        buy_token_2_total_amount,
    )?;

    emit!(OfferMadeTwo {
        offer_id,
        boss: ctx.accounts.boss.key(),
        buy_token_1_total_amount,
        buy_token_2_total_amount,
        sell_token_total_amount,
    });

    Ok(())
}

/// Creates an offer with one buy token.
///
/// Initializes an offer where the boss provides one buy token in exchange for a sell token.
/// Transfers the specified amount of the buy token from the boss to the offer’s account and emits
/// an `OfferMadeOne` event for traceability.
///
/// # Arguments
/// - `ctx`: Context containing the accounts for the offer.
/// - `offer_id`: Unique identifier for the offer, used in PDA derivation.
/// - `buy_token_1_total_amount`: Total amount of buy token 1 to be offered.
/// - `sell_token_total_amount`: Total amount of sell token expected in exchange.
///
/// # Errors
/// - [`MakeOfferErrorCode::InsufficientBalance`] if the boss lacks sufficient buy token amount.
/// - [`MakeOfferErrorCode::InvalidAmount`] if the transfer amount is zero.
pub fn make_offer_one(
    ctx: Context<MakeOfferOne>,
    offer_id: u64,
    buy_token_1_total_amount: u64,
    sell_token_total_amount: u64,
) -> Result<()> {
    require!(
        ctx.accounts.boss_buy_token_1_account.amount >= buy_token_1_total_amount,
        MakeOfferErrorCode::InsufficientBalance
    );
    let offer = &mut ctx.accounts.offer;
    offer.offer_id = offer_id;
    offer.sell_token_mint = ctx.accounts.sell_token_mint.key();
    offer.buy_token_mint_1 = ctx.accounts.buy_token_1_mint.key();
    offer.buy_token_1_total_amount = buy_token_1_total_amount;
    offer.sell_token_total_amount = sell_token_total_amount;
    offer.authority_bump = ctx.bumps.offer_token_authority;
    offer.buy_token_mint_2 = system_program::ID;
    offer.buy_token_2_total_amount = 0;

    transfer_token(
        &ctx,
        &ctx.accounts.boss_buy_token_1_account,
        &ctx.accounts.offer_buy_token_1_account,
        buy_token_1_total_amount,
    )?;

    emit!(OfferMadeOne {
        offer_id,
        boss: ctx.accounts.boss.key(),
        buy_token_1_total_amount,
        sell_token_total_amount,
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
/// - [`MakeOfferErrorCode::InvalidAmount`] if the amount is zero.
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

/// Error codes for offer creation operations.
#[error_code]
pub enum MakeOfferErrorCode {
    /// Triggered when the boss’s token account doesn’t have sufficient balance for the transfer.
    #[msg("Insufficient token balance in boss account")]
    InsufficientBalance,

    /// Triggered when the token transfer amount is zero.
    #[msg("Token transfer amount must be greater than zero")]
    InvalidAmount,
}
