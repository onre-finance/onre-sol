use anchor_lang::prelude::*;
use crate::instructions::buy_offer::BuyOfferAccount;
use crate::state::State;

/// Account structure for initializing the buy offers account.
///
/// This struct defines the accounts required to initialize the buy offers account
/// separately from the main program state. Only the boss can call this.
#[derive(Accounts)]
pub struct InitializeOffers<'info> {
    /// The buy offer account to initialize, rent paid by `boss`.
    #[account(
        init,
        payer = boss,
        space = 8 + std::mem::size_of::<BuyOfferAccount>(),
        seeds = [b"buy_offers"],
        bump
    )]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

    /// The signer authorizing the initialization, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Initializes the buy offers account.
///
/// Creates and initializes the buy offers account with counter set to 0.
/// Only the boss can call this instruction.
///
/// # Arguments
/// - `ctx`: Context containing the accounts to initialize the buy offers.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn initialize_offers(ctx: Context<InitializeOffers>) -> Result<()> {
    // Load the zero-copy account and initialize it
    let mut buy_offer_account = ctx.accounts.buy_offer_account.load_init()?;
    buy_offer_account.counter = 0;

    Ok(())
}