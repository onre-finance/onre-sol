use anchor_lang::prelude::*;
use crate::constants::seeds;
use crate::instructions::buy_offer::BuyOfferAccount;
use crate::instructions::{SingleRedemptionOfferAccount, DualRedemptionOfferAccount};
use crate::state::{State};

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
        space = 8 + std::mem::size_of::<BuyOfferAccount>(),
        seeds = [seeds::BUY_OFFERS],
        bump
    )]
    pub buy_offer_account: AccountLoader<'info, BuyOfferAccount>,

    #[account(
        init,
        payer = boss,
        space = 8 + std::mem::size_of::<SingleRedemptionOfferAccount>(),
        seeds = [seeds::SINGLE_REDEMPTION_OFFERS],
        bump
    )]
    pub single_redemption_offer_account: AccountLoader<'info, SingleRedemptionOfferAccount>,

    /// The dual redemption offer account to initialize, rent paid by `boss`.
    #[account(
        init,
        payer = boss,
        space = 8 + std::mem::size_of::<DualRedemptionOfferAccount>(),
        seeds = [seeds::DUAL_REDEMPTION_OFFERS],
        bump
    )]
    pub dual_redemption_offer_account: AccountLoader<'info, DualRedemptionOfferAccount>,

    /// The signer authorizing the initialization, must be the boss.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Program state, ensures `boss` is authorized.
    #[account(has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

/// Initializes all offer accounts.
///
/// Creates and initializes all offer accounts with counters set to 0.
/// Only the boss can call this instruction.
///
/// # Arguments
/// - `ctx`: Context containing the accounts to initialize the offers.
///
/// # Returns
/// A `Result` indicating success or failure.
pub fn initialize_offers(ctx: Context<InitializeOffers>) -> Result<()> {
    let mut buy_offer_account = ctx.accounts.buy_offer_account.load_init()?;
    buy_offer_account.counter = 0;
    let mut single_redemption_offer_account = ctx.accounts.single_redemption_offer_account.load_init()?;
    single_redemption_offer_account.counter = 0;
    let mut dual_redemption_offer_account = ctx.accounts.dual_redemption_offer_account.load_init()?;
    dual_redemption_offer_account.counter = 0;
    
    msg!("Offers accounts initialized - buy_offers, single_redemption_offers, and dual_redemption_offers counters set to 0");
    
    Ok(())
}