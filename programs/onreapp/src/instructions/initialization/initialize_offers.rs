use crate::constants::seeds;
use crate::instructions::offer::OfferAccount;
use crate::state::State;
use anchor_lang::prelude::*;

/// Account structure for initializing the offer accounts.
///
/// This struct defines the accounts required to initialize all offer accounts
/// separately from the main program state. Only the boss can call this.
#[derive(Accounts)]
pub struct InitializeOffers<'info> {
    /// The buy offer account to initialize, rent paid by `boss`.
    #[account(
        init,
        payer = boss,
        space = 8 + std::mem::size_of::<OfferAccount>(),
        seeds = [seeds::OFFERS],
        bump
    )]
    pub offer_account: AccountLoader<'info, OfferAccount>,

    /// The signer authorizing the initialization, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Initializes the offer account.
///
/// Creates and initializes the offer account with counter set to 0.
/// Only the boss can call this instruction.
///
/// # Arguments
/// - `ctx`: Context containing the accounts to initialize the offers.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn initialize_offers(ctx: Context<InitializeOffers>) -> Result<()> {
    let mut buy_offer_account = ctx.accounts.offer_account.load_init()?;
    buy_offer_account.counter = 0;

    msg!("Offer account initialized");

    Ok(())
}
