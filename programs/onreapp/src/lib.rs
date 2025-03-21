use anchor_lang::prelude::*;
use instructions::*;

// Program ID declaration
declare_id!("J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2");

pub mod contexts;
pub mod instructions;
pub mod state;

/// The main program module for the Onre App.
///
/// This module defines the entry points for all program instructions, delegating to specific
/// instruction modules for execution. It manages offers where a boss provides buy tokens
/// in exchange for sell tokens, with functionality for making, taking, and closing offers,
/// as well as managing the program state.
///
/// # Security
/// - Instructions are secured by constraints like `has_one = boss` and PDA derivation.
/// - Events are emitted in instruction modules for state changes (e.g., offer creation, closure).
#[program]
pub mod onre_app {
    use super::*;

    /// Creates an offer with one buy token.
    ///
    /// Delegates to `make_offer::make_offer_one` to initialize an offer with a single buy token.
    /// Emits an `OfferMadeOne` event upon success.
    pub fn make_offer_one(
        ctx: Context<MakeOfferOne>,
        offer_id: u64,
        buy_token_1_total_amount: u64,
        sell_token_total_amount: u64,
    ) -> Result<()> {
        make_offer::make_offer_one(
            ctx,
            offer_id,
            buy_token_1_total_amount,
            sell_token_total_amount,
        )
    }

    /// Creates an offer with two buy tokens.
    ///
    /// Delegates to `make_offer::make_offer_two` to initialize an offer with two buy tokens.
    /// Emits an `OfferMadeTwo` event upon success.
    pub fn make_offer_two(
        ctx: Context<MakeOfferTwo>,
        offer_id: u64,
        buy_token_1_total_amount: u64,
        buy_token_2_total_amount: u64,
        sell_token_total_amount: u64,
    ) -> Result<()> {
        make_offer::make_offer_two(
            ctx,
            offer_id,
            buy_token_1_total_amount,
            buy_token_2_total_amount,
            sell_token_total_amount,
        )
    }

    /// Closes an offer with one buy token.
    ///
    /// Delegates to `close_offer::close_offer_one` to transfer remaining tokens and close the offer.
    /// Emits `TokensTransferred` and `OfferClosed` events.
    pub fn close_offer_one(ctx: Context<CloseOfferOne>) -> Result<()> {
        close_offer::close_offer_one(ctx)
    }

    /// Closes an offer with two buy tokens.
    ///
    /// Delegates to `close_offer::close_offer_two` to transfer remaining tokens and close the offer.
    /// Emits `TokensTransferred` and `OfferClosed` events.
    pub fn close_offer_two(ctx: Context<CloseOfferTwo>) -> Result<()> {
        close_offer::close_offer_two(ctx)
    }

    /// Initializes the program state.
    ///
    /// Delegates to `initialize::initialize` to set the initial boss in the state account.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::initialize(ctx)
    }

    /// Updates the boss in the program state.
    ///
    /// Delegates to `set_boss::set_boss` to change the boss, emitting a `BossUpdated` event.
    pub fn set_boss(ctx: Context<SetBoss>, new_boss: Pubkey) -> Result<()> {
        set_boss::set_boss(ctx, new_boss)
    }

    /// Takes an offer with one buy token.
    ///
    /// Delegates to `take_offer::take_offer_one` to exchange sell tokens for one buy token.
    /// Emits an `OfferTakenOne` event.
    pub fn take_offer_one(ctx: Context<TakeOfferOne>, sell_token_amount: u64) -> Result<()> {
        take_offer::take_offer_one(ctx, sell_token_amount)
    }

    /// Takes an offer with two buy tokens.
    ///
    /// Delegates to `take_offer::take_offer_two` to exchange sell tokens for two buy tokens.
    /// Emits an `OfferTakenTwo` event.
    pub fn take_offer_two(ctx: Context<TakeOfferTwo>, sell_token_amount: u64) -> Result<()> {
        take_offer::take_offer_two(ctx, sell_token_amount)
    }
}
