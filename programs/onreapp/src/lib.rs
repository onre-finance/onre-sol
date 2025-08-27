use anchor_lang::prelude::*;
use instructions::*;

// Program ID declaration
declare_id!("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe");

pub mod contexts;
pub mod instructions;
pub mod state;

/// The main program module for the Onre App.
///
/// This module defines the entry points for all program instructions. It facilitates the creation
/// and management of offers where a "boss" provides one or two types of buy tokens in exchange for
/// sell tokens. A key feature is the dynamic pricing model for offers, where the amount of
/// sell token required can change over the offer's duration based on predefined parameters.
///
/// Core functionalities include:
/// - Making offers with dynamic pricing (`make_offer_one`, `make_offer_two`).
/// - Taking offers, respecting the current price (`take_offer_one`, `take_offer_two`).
/// - Closing offers (`close_offer_one`, `close_offer_two`).
/// - Program state initialization and boss management (`initialize`, `set_boss`).
///
/// # Dynamic Pricing Model
/// The price (amount of sell tokens per buy token) is determined by:
/// - `sell_token_start_amount`: Sell token amount at the beginning of the offer.
/// - `sell_token_end_amount`: Sell token amount at the end of the offer.
/// - `offer_start_time`, `offer_end_time`: Defines the offer's active duration.
/// - `price_fix_duration`: The duration of each discrete pricing interval within the offer period.
/// The price interpolates linearly across these intervals.
///
/// # Security
/// - Access controls are enforced, for example, ensuring only the `boss` can create offers or update critical state.
/// - PDA (Program Derived Address) accounts are used for offer and token authorities, ensuring ownership.
/// - Events are emitted for significant actions (e.g., `OfferMadeOne`, `OfferTakenTwo`) for off-chain traceability.
#[program]
pub mod onre_app {
    use super::*;

    /// Creates an offer with one buy token.
    ///
    /// Delegates to `make_offer::make_offer_one`.
    /// The price of the sell token changes over time based on `sell_token_start_amount`,
    /// `sell_token_end_amount`, and `price_fix_duration` within the offer's active time window.
    /// Emits an `OfferMadeOne` event upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `MakeOfferOne`.
    /// - `offer_id`: Unique ID for the offer.
    /// - `buy_token_total_amount`: Total amount of the buy token offered.
    /// - `sell_token_start_amount`: Sell token amount at the start of the offer.
    /// - `sell_token_end_amount`: Sell token amount at the end of the offer.
    /// - `offer_start_time`: Offer activation timestamp.
    /// - `offer_end_time`: Offer expiration timestamp.
    /// - `price_fix_duration`: Duration of each price interval.
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
        make_offer::make_offer_one(
            ctx,
            offer_id,
            buy_token_total_amount,
            sell_token_start_amount,
            sell_token_end_amount,
            offer_start_time,
            offer_end_time,
            price_fix_duration,
        )
    }

    /// Creates an offer with two buy tokens.
    ///
    /// Delegates to `make_offer::make_offer_two`.
    /// The price of the sell token changes over time based on `sell_token_start_amount`,
    /// `sell_token_end_amount`, and `price_fix_duration` within the offer's active time window.
    /// Emits an `OfferMadeTwo` event upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `MakeOfferTwo`.
    /// - `offer_id`: Unique ID for the offer.
    /// - `buy_token_1_total_amount`: Total amount of the first buy token offered.
    /// - `buy_token_2_total_amount`: Total amount of the second buy token offered.
    /// - `sell_token_start_amount`: Sell token amount at the start of the offer.
    /// - `sell_token_end_amount`: Sell token amount at the end of the offer.
    /// - `offer_start_time`: Offer activation timestamp.
    /// - `offer_end_time`: Offer expiration timestamp.
    /// - `price_fix_duration`: Duration of each price interval.
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
        make_offer::make_offer_two(
            ctx,
            offer_id,
            buy_token_1_total_amount,
            buy_token_2_total_amount,
            sell_token_start_amount,
            sell_token_end_amount,
            offer_start_time,
            offer_end_time,
            price_fix_duration,
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

    /// Initializes a permissionless account.
    ///
    /// Delegates to `initialize::initialize_permissionless_account` to create a new permissionless account.
    /// The account is created as a PDA with the seed "permissionless-1".
    /// Only the boss can initialize permissionless accounts.
    pub fn initialize_permissionless_account(
        ctx: Context<InitializePermissionlessAccount>,
        name: String,
    ) -> Result<()> {
        initialize::initialize_permissionless_account(ctx, name)
    }

    /// Updates the boss in the program state.
    ///
    /// Delegates to `set_boss::set_boss` to change the boss, emitting a `BossUpdated` event.
    pub fn set_boss(ctx: Context<SetBoss>, new_boss: Pubkey) -> Result<()> {
        set_boss::set_boss(ctx, new_boss)
    }

    /// Takes an offer with one buy token, respecting the current dynamic price.
    ///
    /// Delegates to `take_offer::take_offer_one`.
    /// The amount of buy token received is calculated based on the current price derived from the
    /// offer's dynamic pricing parameters.
    /// Emits an `OfferTakenOne` event.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TakeOfferOne`.
    /// - `sell_token_amount`: Amount of sell tokens the user provides.
    pub fn take_offer_one(ctx: Context<TakeOfferOne>, sell_token_amount: u64) -> Result<()> {
        take_offer::take_offer_one(ctx, sell_token_amount)
    }

    /// Takes an offer with two buy tokens, respecting the current dynamic price.
    ///
    /// Delegates to `take_offer::take_offer_two`.
    /// The amount of each buy token received is calculated based on the current price derived from the
    /// offer's dynamic pricing parameters.
    /// Emits an `OfferTakenTwo` event.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TakeOfferTwo`.
    /// - `sell_token_amount`: Amount of sell tokens the user provides.
    pub fn take_offer_two(ctx: Context<TakeOfferTwo>, sell_token_amount: u64) -> Result<()> {
        take_offer::take_offer_two(ctx, sell_token_amount)
    }

    /// Takes an offer with one buy token via permissionless route.
    ///
    /// Delegates to `take_offer_one_permissionless::take_offer_one_permissionless`.
    /// This instruction creates a program-controlled intermediary token account,
    /// routes tokens from offer -> intermediary -> user, then closes the intermediary account.
    /// The economic outcome is identical to `take_offer_one`, but provides an additional
    /// layer of indirection through the program-controlled intermediary account.
    /// Emits an `OfferTakenOnePermissionless` event.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TakeOfferOnePermissionless`.
    /// - `sell_token_amount`: Amount of sell tokens the user provides.
    pub fn take_offer_one_permissionless(
        ctx: Context<TakeOfferOnePermissionless>,
        sell_token_amount: u64,
    ) -> Result<()> {
        take_offer_one_permissionless::take_offer_one_permissionless(ctx, sell_token_amount)
    }
}
