use anchor_lang::prelude::*;
use instructions::*;

// Program ID declaration
declare_id!("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe");

pub mod constants;
pub mod contexts;
pub mod instructions;
pub mod state;
pub mod utils;

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
pub mod onreapp {
    use super::*;

    /// Initializes the buy offers account.
    ///
    /// Delegates to `buy_offer::initialize_offers`.
    /// Only the boss can call this instruction to create the buy offers account.
    ///
    /// # Arguments
    /// - `ctx`: Context for `InitializeOffers`.
    pub fn initialize_offers(ctx: Context<InitializeOffers>) -> Result<()> {
        initialize_offers::initialize_offers(ctx)
    }

    pub fn initialize_vault_authority(ctx: Context<InitializeVaultAuthority>) -> Result<()> {
        initialize_vault_authority::initialize_vault_authority(ctx)
    }

    /// Deposits tokens into the vault.
    ///
    /// Delegates to `vault_operations::vault_deposit`.
    /// Transfers tokens from boss's account to vault's token account for the specified mint.
    /// Creates vault token account if it doesn't exist using init_if_needed.
    /// Only the boss can call this instruction.
    ///
    /// # Arguments
    /// - `ctx`: Context for `VaultDeposit`.
    /// - `amount`: Amount of tokens to deposit.
    pub fn vault_deposit(ctx: Context<VaultDeposit>, amount: u64) -> Result<()> {
        vault_operations::vault_deposit(ctx, amount)
    }

    /// Withdraws tokens from the vault.
    ///
    /// Delegates to `vault_operations::vault_withdraw`.
    /// Transfers tokens from vault's token account to boss's token account for the specified mint.
    /// Both token accounts must already exist.
    /// Only the boss can call this instruction.
    ///
    /// # Arguments
    /// - `ctx`: Context for `VaultWithdraw`.
    /// - `amount`: Amount of tokens to withdraw.
    pub fn vault_withdraw(ctx: Context<VaultWithdraw>, amount: u64) -> Result<()> {
        vault_operations::vault_withdraw(ctx, amount)
    }

    /// Creates a buy offer.
    ///
    /// Delegates to `buy_offer::make_buy_offer`.
    /// The price of the token_out changes over time based on `start_price`,
    /// `end_price`, and `price_fix_duration` within the offer's active time window.
    /// Emits a `BuyOfferMade` event upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `MakeBuyOffer`.
    /// - `offer_id`: Unique ID for the offer.
    pub fn make_buy_offer(ctx: Context<MakeBuyOffer>) -> Result<()> {
        buy_offer::make_buy_offer(ctx)
    }

    /// Creates a single redemption offer.
    ///
    /// Delegates to `redemption_offer::make_single_redemption_offer`.
    /// Creates an offer where users can exchange token_in for token_out at a fixed price.
    /// Emits a `SingleRedemptionOfferMadeEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `MakeSingleRedemptionOffer`.
    /// - `start_time`: Unix timestamp for when the offer becomes active.
    /// - `end_time`: Unix timestamp for when the offer expires.
    /// - `price`: How much token_in needed for 1 token_out, with 9 decimal precision.
    pub fn make_single_redemption_offer(
        ctx: Context<MakeSingleRedemptionOffer>,
        start_time: u64,
        end_time: u64,
        price: u64,
    ) -> Result<()> {
        redemption_offer::make_single_redemption_offer(ctx, start_time, end_time, price)
    }

    /// Closes a single redemption offer.
    ///
    /// Delegates to `redemption_offer::close_single_redemption_offer`.
    /// Removes the offer from the single redemption offers account and clears its data.
    /// Emits a `CloseSingleRedemptionOfferEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `CloseSingleRedemptionOffer`.
    /// - `offer_id`: ID of the offer to close.
    pub fn close_single_redemption_offer(
        ctx: Context<CloseSingleRedemptionOffer>,
        offer_id: u64,
    ) -> Result<()> {
        redemption_offer::close_single_redemption_offer(ctx, offer_id)
    }

    /// Takes a single redemption offer.
    ///
    /// Delegates to `redemption_offer::take_single_redemption_offer`.
    /// Allows a user to exchange token_in for token_out based on the offer's price.
    /// Price is stored with token_in_decimals precision. Anyone can take the offer.
    /// Emits a `TakeSingleRedemptionOfferEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TakeSingleRedemptionOffer`.
    /// - `offer_id`: ID of the offer to take.
    /// - `token_in_amount`: Amount of token_in to provide.
    pub fn take_single_redemption_offer(
        ctx: Context<TakeSingleRedemptionOffer>,
        offer_id: u64,
        token_in_amount: u64,
    ) -> Result<()> {
        redemption_offer::take_single_redemption_offer(ctx, offer_id, token_in_amount)
    }

    /// Creates a dual redemption offer.
    ///
    /// Delegates to `redemption_offer::make_dual_redemption_offer`.
    /// Creates an offer where users can exchange token_in for two different token_out at fixed prices.
    /// The ratio_basis_points determines the split between the two output tokens.
    /// Emits a `DualRedemptionOfferMadeEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `MakeDualRedemptionOffer`.
    /// - `start_time`: Unix timestamp for when the offer becomes active.
    /// - `end_time`: Unix timestamp for when the offer expires.
    /// - `price_1`: Fixed price for token_out_1 with 9 decimal precision.
    /// - `price_2`: Fixed price for token_out_2 with 9 decimal precision.
    /// - `ratio_basis_points`: Ratio in basis points for token_out_1 (e.g., 8000 = 80% for token_out_1, 20% for token_out_2).
    pub fn make_dual_redemption_offer(
        ctx: Context<MakeDualRedemptionOffer>,
        start_time: u64,
        end_time: u64,
        price_1: u64,
        price_2: u64,
        ratio_basis_points: u64,
    ) -> Result<()> {
        redemption_offer::make_dual_redemption_offer(
            ctx,
            start_time,
            end_time,
            price_1,
            price_2,
            ratio_basis_points,
        )
    }

    /// Closes a dual redemption offer.
    ///
    /// Delegates to `redemption_offer::close_dual_redemption_offer`.
    /// Removes the offer from the dual redemption offers account and clears its data.
    /// Only the boss can close dual redemption offers.
    /// Emits a `CloseDualRedemptionOfferEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `CloseDualRedemptionOffer`.
    /// - `offer_id`: ID of the offer to close.
    pub fn close_dual_redemption_offer(
        ctx: Context<CloseDualRedemptionOffer>,
        offer_id: u64,
    ) -> Result<()> {
        redemption_offer::close_dual_redemption_offer(ctx, offer_id)
    }

    /// Takes a dual redemption offer.
    ///
    /// Delegates to `redemption_offer::take_dual_redemption_offer`.
    /// Allows a user to exchange token_in for token_out_1 and token_out_2 based on the offer's prices and ratio.
    /// The ratio_basis_points determines how the token_in amount is split between the two output tokens.
    /// Anyone can take the offer as long as it's active and vault has sufficient balances.
    /// Emits a `TakeDualRedemptionOfferEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TakeDualRedemptionOffer`.
    /// - `offer_id`: ID of the offer to take.
    /// - `token_in_amount`: Amount of token_in to provide.
    pub fn take_dual_redemption_offer(
        ctx: Context<TakeDualRedemptionOffer>,
        offer_id: u64,
        token_in_amount: u64,
    ) -> Result<()> {
        redemption_offer::take_dual_redemption_offer(ctx, offer_id, token_in_amount)
    }

    /// Closes a buy offer.
    ///
    /// Delegates to `buy_offer::close_buy_offer`.
    /// Removes the offer from the buy offers account and clears its data.
    /// Emits a `CloseBuyOfferEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `CloseBuyOffer`.
    /// - `offer_id`: ID of the offer to close.
    pub fn close_buy_offer(ctx: Context<CloseBuyOffer>, offer_id: u64) -> Result<()> {
        buy_offer::close_buy_offer(ctx, offer_id)
    }

    /// Adds a time vector to an existing buy offer.
    ///
    /// Delegates to `buy_offer::add_buy_offer_time_vector`.
    /// Creates a new time vector with auto-generated vector_id for the specified buy offer.
    /// Emits a `BuyOfferVectorAdded` event upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `AddBuyOfferVector`.
    /// - `offer_id`: ID of the buy offer to add the vector to.
    /// - `start_time`: Unix timestamp when the vector becomes active.
    /// - `start_price`: Price at the beginning of the vector.
    /// - `price_yield`: Price yield percentage * 10000 (with 4 decimal places).
    /// - `price_fix_duration`: Duration in seconds for each price interval.
    pub fn add_buy_offer_vector(
        ctx: Context<AddBuyOfferVector>,
        offer_id: u64,
        start_time: u64,
        start_price: u64,
        price_yield: u64,
        price_fix_duration: u64,
    ) -> Result<()> {
        buy_offer::add_buy_offer_vector(
            ctx,
            offer_id,
            start_time,
            start_price,
            price_yield,
            price_fix_duration,
        )
    }

    /// Deletes a time vector from a buy offer.
    ///
    /// Delegates to `buy_offer::delete_buy_offer_vector`.
    /// Removes the specified time vector from the buy offer by setting it to default values.
    /// Only the boss can delete time vectors from offers.
    /// Emits a `BuyOfferVectorDeleted` event upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `DeleteBuyOfferVector`.
    /// - `offer_id`: ID of the buy offer containing the vector to delete.
    /// - `vector_id`: ID of the vector to delete.
    pub fn delete_buy_offer_vector(
        ctx: Context<DeleteBuyOfferVector>,
        offer_id: u64,
        vector_id: u64,
    ) -> Result<()> {
        buy_offer::delete_buy_offer_vector(ctx, offer_id, vector_id)
    }

    /// Takes a buy offer.
    ///
    /// Delegates to `buy_offer::take_buy_offer`.
    /// Allows a user to exchange token_in for token_out based on the offer's dynamic price.
    /// Emits a `TakeBuyOfferEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TakeBuyOffer`.
    /// - `offer_id`: ID of the offer to take.
    /// - `token_in_amount`: Amount of token_in to provide.
    pub fn take_buy_offer(
        ctx: Context<TakeBuyOffer>,
        offer_id: u64,
        token_in_amount: u64,
    ) -> Result<()> {
        buy_offer::take_buy_offer(ctx, offer_id, token_in_amount)
    }

    /// Initializes the program state.
    ///
    /// Delegates to `initialize::initialize` to set the initial boss in the state account.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::initialize(ctx)
    }

    // /// Updates the boss in the program state.
    // ///
    /// Delegates to `set_boss::set_boss` to change the boss, emitting a `BossUpdated` event.
    pub fn set_boss(ctx: Context<SetBoss>, new_boss: Pubkey) -> Result<()> {
        set_boss::set_boss(ctx, new_boss)
    }
}
