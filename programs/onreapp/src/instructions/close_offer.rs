use crate::contexts::CloseOfferContext;
use crate::state::{Offer, State};
use anchor_lang::prelude::*;
use anchor_lang::{system_program, Accounts};
use anchor_spl::token;
use anchor_spl::token::{CloseAccount, Token, TokenAccount, Transfer};

/// Event emitted when tokens are transferred during offer closure.
#[event]
pub struct TokensTransferred {
    pub offer_id: u64,
    pub from_account: Pubkey,
    pub to_account: Pubkey,
    pub amount: u64,
}

/// Event emitted when an offer is closed.
#[event]
pub struct OfferClosed {
    pub offer_id: u64,
    pub boss: Pubkey,
    pub num_buy_tokens: u8, // 1 for CloseOfferOne, 2 for CloseOfferTwo
}

/// Account structure for closing an offer with one buy token.
///
/// This struct defines the accounts required to close an offer involving a single buy token,
/// transferring remaining tokens to the boss and closing associated accounts.
///
/// # Preconditions
/// - All Associated Token Accounts (ATAs) must be created and initialized prior to execution.
///   This includes `offer_sell_token_account`, `offer_buy_1_token_account`, `boss_sell_token_account`,
///   and `boss_buy_1_token_account`.
#[derive(Accounts)]
pub struct CloseOfferOne<'info> {
    /// The offer account to be closed, with rent refunded to `boss`.
    #[account(mut, close = boss)]
    pub offer: Account<'info, Offer>,

    /// Offer's sell token ATA, must exist prior to execution, controlled by `offer_token_authority`.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = offer_token_authority,
    )]
    pub offer_sell_token_account: Account<'info, TokenAccount>,

    /// Offer's buy token 1 ATA, must exist prior to execution, controlled by `offer_token_authority`.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_mint_1,
        associated_token::authority = offer_token_authority,
    )]
    pub offer_buy_1_token_account: Account<'info, TokenAccount>,

    /// Boss's buy token 1 ATA, must exist prior to execution, owned by `boss`.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_mint_1,
        associated_token::authority = boss,
    )]
    pub boss_buy_1_token_account: Account<'info, TokenAccount>,

    /// Boss's sell token ATA, must exist prior to execution, owned by `boss`.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = boss,
    )]
    pub boss_sell_token_account: Account<'info, TokenAccount>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// Derived PDA for token authority, does not store data.
    ///
    /// # Note
    /// This account is marked with `CHECK` as it’s validated by the seed derivation.
    #[account(seeds = [b"offer_authority", offer.offer_id.to_le_bytes().as_ref()], bump)]
    pub offer_token_authority: AccountInfo<'info>,

    /// The signer authorizing the closure, typically the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// SPL Token program for token operations.
    pub token_program: Program<'info, Token>,

    /// Solana System program for account operations.
    pub system_program: Program<'info, System>,
}

/// Closes a single buy token offer.
///
/// Transfers remaining sell and buy tokens to the boss’s accounts, closes the offer’s token accounts,
/// and refunds the `offer` account’s rent to `boss`. Emits events for token transfers and offer closure.
///
/// # Errors
/// - [`CloseOfferErrorCode::InvalidCloseOffer`] if `buy_token_mint_2 != System Program ID`.
/// - [`CloseOfferErrorCode::InvalidMint`] if token account mints mismatch during transfers.
pub fn close_offer_one(ctx: Context<CloseOfferOne>) -> Result<()> {
    require!(
        ctx.accounts.offer.buy_token_mint_2 == system_program::ID,
        CloseOfferErrorCode::InvalidCloseOffer
    );

    let offer_sell_token_account = &ctx.accounts.offer_sell_token_account;
    let offer_buy_1_token_account = &ctx.accounts.offer_buy_1_token_account;
    let boss_sell_token_account = &ctx.accounts.boss_sell_token_account;
    let boss_buy_1_token_account = &ctx.accounts.boss_buy_1_token_account;

    transfer_remaining_tokens(&ctx, offer_sell_token_account, boss_sell_token_account)?;
    transfer_remaining_tokens(&ctx, offer_buy_1_token_account, boss_buy_1_token_account)?;

    let offer_id_bytes = &ctx.accounts.offer.offer_id.to_le_bytes();
    let seeds = &[
        b"offer_authority".as_ref(),
        offer_id_bytes.as_ref(),
        &[ctx.accounts.offer.authority_bump],
    ];
    let signer_seeds = &[seeds.as_ref()];

    emit!(OfferClosed {
        offer_id: ctx.accounts.offer.offer_id,
        boss: ctx.accounts.boss.key(),
        num_buy_tokens: 1,
    });

    close_token_account(
        ctx.accounts.offer_sell_token_account.clone(),
        ctx.accounts.offer_token_authority.clone(),
        ctx.accounts.boss.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        signer_seeds,
    )?;

    close_token_account(
        ctx.accounts.offer_buy_1_token_account.clone(),
        ctx.accounts.offer_token_authority.clone(),
        ctx.accounts.boss.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        signer_seeds,
    )?;

    Ok(())
}

/// Account structure for closing an offer with two buy tokens.
///
/// This struct defines the accounts required to close an offer involving two buy tokens,
/// transferring remaining tokens to the boss and closing associated accounts.
///
/// # Preconditions
/// - All Associated Token Accounts (ATAs) must be created and initialized prior to execution.
///   This includes `offer_sell_token_account`, `offer_buy_1_token_account`, `offer_buy_2_token_account`,
///   `boss_sell_token_account`, `boss_buy_1_token_account`, and `boss_buy_2_token_account`.
#[derive(Accounts)]
pub struct CloseOfferTwo<'info> {
    /// The offer account to be closed, with rent refunded to `boss`.
    #[account(mut, close = boss)]
    pub offer: Account<'info, Offer>,

    /// Offer's sell token ATA, must exist prior to execution, controlled by `offer_token_authority`.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_sell_token_account: Account<'info, TokenAccount>,

    /// Offer's buy token 1 ATA, must exist prior to execution, controlled by `offer_token_authority`.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_mint_1,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_1_token_account: Account<'info, TokenAccount>,

    /// Offer's buy token 2 ATA, must exist prior to execution, controlled by `offer_token_authority`.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_mint_2,
        associated_token::authority = offer_token_authority,
  )]
    pub offer_buy_2_token_account: Account<'info, TokenAccount>,

    /// Boss's buy token 1 ATA, must exist prior to execution, owned by `boss`.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_mint_1,
        associated_token::authority = boss,
  )]
    pub boss_buy_1_token_account: Account<'info, TokenAccount>,

    /// Boss's buy token 2 ATA, must exist prior to execution, owned by `boss`.
    #[account(
        mut,
        associated_token::mint = offer.buy_token_mint_2,
        associated_token::authority = boss,
  )]
    pub boss_buy_2_token_account: Account<'info, TokenAccount>,

    /// Boss's sell token ATA, must exist prior to execution, owned by `boss`.
    #[account(
        mut,
        associated_token::mint = offer.sell_token_mint,
        associated_token::authority = boss,
  )]
    pub boss_sell_token_account: Account<'info, TokenAccount>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Account<'info, State>,

    /// Derived PDA for token authority, does not store data.
    ///
    /// # Note
    /// This account is marked with `CHECK` as it’s validated by the seed derivation.
    #[account(seeds = [b"offer_authority", offer.offer_id.to_le_bytes().as_ref()], bump)]
    pub offer_token_authority: AccountInfo<'info>,

    /// The signer authorizing the closure, typically the boss.
    pub boss: Signer<'info>,

    /// SPL Token program for token operations.
    pub token_program: Program<'info, Token>,

    /// Solana System program for account operations.
    pub system_program: Program<'info, System>,
}

/// Closes a dual buy token offer.
///
/// Transfers remaining sell and buy tokens to the boss’s accounts, closes the offer’s token accounts,
/// and refunds the `offer` account’s rent to `boss`. Emits events for token transfers and offer closure.
///
/// # Errors
/// - [`CloseOfferErrorCode::InvalidMint`] if token account mints mismatch during transfers.
pub fn close_offer_two(ctx: Context<CloseOfferTwo>) -> Result<()> {
    let offer_sell_token_account = &ctx.accounts.offer_sell_token_account;
    let offer_buy_1_token_account = &ctx.accounts.offer_buy_1_token_account;
    let offer_buy_2_token_account = &ctx.accounts.offer_buy_2_token_account;

    let boss_sell_token_account = &ctx.accounts.boss_sell_token_account;
    let boss_buy_1_token_account = &ctx.accounts.boss_buy_1_token_account;
    let boss_buy_2_token_account = &ctx.accounts.boss_buy_2_token_account;

    transfer_remaining_tokens(&ctx, offer_sell_token_account, boss_sell_token_account)?;
    transfer_remaining_tokens(&ctx, offer_buy_1_token_account, boss_buy_1_token_account)?;
    transfer_remaining_tokens(&ctx, offer_buy_2_token_account, boss_buy_2_token_account)?;

    let offer_id_bytes = &ctx.accounts.offer.offer_id.to_le_bytes();
    let seeds = &[
        b"offer_authority".as_ref(),
        offer_id_bytes.as_ref(),
        &[ctx.accounts.offer.authority_bump],
    ];
    let signer_seeds = &[seeds.as_ref()];

    emit!(OfferClosed {
        offer_id: ctx.accounts.offer.offer_id,
        boss: ctx.accounts.boss.key(),
        num_buy_tokens: 2,
    });
    close_token_account(
        ctx.accounts.offer_sell_token_account.clone(),
        ctx.accounts.offer_token_authority.clone(),
        ctx.accounts.boss.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        signer_seeds,
    )?;

    close_token_account(
        ctx.accounts.offer_buy_1_token_account.clone(),
        ctx.accounts.offer_token_authority.clone(),
        ctx.accounts.boss.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        signer_seeds,
    )?;

    close_token_account(
        ctx.accounts.offer_buy_2_token_account.clone(),
        ctx.accounts.offer_token_authority.clone(),
        ctx.accounts.boss.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        signer_seeds,
    )?;

    Ok(())
}

/// Trait implementation for `CloseOfferOne` to satisfy `CloseOfferContext`.
impl<'info> CloseOfferContext<'info> for CloseOfferOne<'info> {
    fn token_program(&self) -> &Program<'info, Token> {
        &self.token_program
    }

    fn offer_token_authority(&self) -> &AccountInfo<'info> {
        &self.offer_token_authority
    }

    fn offer(&self) -> &Account<'info, Offer> {
        &self.offer
    }
}

/// Trait implementation for `CloseOfferTwo` to satisfy `CloseOfferContext`.
impl<'info> CloseOfferContext<'info> for CloseOfferTwo<'info> {
    fn token_program(&self) -> &Program<'info, Token> {
        &self.token_program
    }

    fn offer_token_authority(&self) -> &AccountInfo<'info> {
        &self.offer_token_authority
    }

    fn offer(&self) -> &Account<'info, Offer> {
        &self.offer
    }
}

/// Transfers remaining tokens from a source to a destination account if a balance exists.
///
/// Emits a `TokensTransferred` event when tokens are moved.
///
/// # Arguments
/// - `ctx`: Context containing program and account information.
/// - `from_token_account`: Source token account to transfer from.
/// - `to_token_account`: Destination token account to transfer to.
///
/// # Errors
/// - [`CloseOfferErrorCode::InvalidMint`] if the mints of source and destination accounts don’t match.
fn transfer_remaining_tokens<'info, T: CloseOfferContext<'info> + anchor_lang::Bumps>(
    ctx: &Context<T>,
    from_token_account: &Account<'info, TokenAccount>,
    to_token_account: &Account<'info, TokenAccount>,
) -> Result<()> {
    require!(
        from_token_account.mint == to_token_account.mint,
        CloseOfferErrorCode::InvalidMint
    );
    let balance = from_token_account.amount;

    if balance > 0 {
        let offer_id_bytes = ctx.accounts.offer().offer_id.to_le_bytes();
        let seeds = &[
            b"offer_authority".as_ref(),
            offer_id_bytes.as_ref(),
            &[ctx.accounts.offer().authority_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_transfer = CpiContext::new_with_signer(
            ctx.accounts.token_program().to_account_info(),
            Transfer {
                from: from_token_account.to_account_info(),
                to: to_token_account.to_account_info(),
                authority: ctx.accounts.offer_token_authority().to_account_info(),
            },
            signer_seeds,
        );

        token::transfer(cpi_transfer, balance)?;

        emit!(TokensTransferred {
            offer_id: ctx.accounts.offer().offer_id,
            from_account: from_token_account.key(),
            to_account: to_token_account.key(),
            amount: balance,
        });
    }

    Ok(())
}

/// Closes a token account and refunds its rent to a destination account.
///
/// # Arguments
/// - `token_account`: The token account to close.
/// - `authority`: The PDA authority controlling the token account.
/// - `destination`: The account receiving the refunded rent.
/// - `token_program`: The SPL Token program instance.
/// - `signer_seeds`: Seeds for signing as the PDA authority.
fn close_token_account<'info>(
    token_account: Account<'info, TokenAccount>,
    authority: AccountInfo<'info>,
    destination: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let cpi_accounts = CloseAccount {
        account: token_account.to_account_info(),
        destination,
        authority,
    };

    let cpi_ctx = CpiContext::new_with_signer(token_program, cpi_accounts, signer_seeds);
    token::close_account(cpi_ctx)
}

/// Error codes for offer closure operations.
#[error_code]
pub enum CloseOfferErrorCode {
    /// Triggered when token account mints do not match during a transfer.
    #[msg("Invalid mint")]
    InvalidMint,

    /// Triggered when the offer type is invalid for the closure instruction.
    #[msg("Invalid close offer")]
    InvalidCloseOffer,
}
