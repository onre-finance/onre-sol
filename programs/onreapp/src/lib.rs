use anchor_lang::prelude::*;
use instructions::*;

// Program ID declaration
declare_id!("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe");

pub mod constants;
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

    /// Initializes the offers account.
    ///
    /// Delegates to `offer::initialize_offers`.
    /// Only the boss can call this instruction to create the offers account.
    ///
    /// # Arguments
    /// - `ctx`: Context for `InitializeOffers`.
    pub fn initialize_offers(ctx: Context<InitializeOffers>) -> Result<()> {
        initialize_offers::initialize_offers(ctx)
    }

    pub fn initialize_vault_authority(ctx: Context<InitializeVaultAuthority>) -> Result<()> {
        initialize_vault_authority::initialize_vault_authority(ctx)
    }

    /// Deposits tokens into the offer vault.
    ///
    /// Delegates to `vault_operations::offer_vault_deposit`.
    /// Transfers tokens from boss's account to offer vault's token account for the specified mint.
    /// Creates vault token account if it doesn't exist using init_if_needed.
    /// Only the boss can call this instruction.
    ///
    /// # Arguments
    /// - `ctx`: Context for `OfferVaultDeposit`.
    /// - `amount`: Amount of tokens to deposit.
    pub fn offer_vault_deposit(ctx: Context<OfferVaultDeposit>, amount: u64) -> Result<()> {
        vault_operations::offer_vault_deposit(ctx, amount)
    }

    /// Withdraws tokens from the offer vault.
    ///
    /// Delegates to `vault_operations::offer_vault_withdraw`.
    /// Transfers tokens from offer vault's token account to boss's account for the specified mint.
    /// Creates boss token account if it doesn't exist using init_if_needed.
    /// Only the boss can call this instruction.
    ///
    /// # Arguments
    /// - `ctx`: Context for `OfferVaultWithdraw`.
    /// - `amount`: Amount of tokens to withdraw.
    pub fn offer_vault_withdraw(ctx: Context<OfferVaultWithdraw>, amount: u64) -> Result<()> {
        vault_operations::offer_vault_withdraw(ctx, amount)
    }

    /// Creates an offer.
    ///
    /// Delegates to `offer::make_offer`.
    /// The price of the token_out changes over time based on `base_price`,
    /// `end_price`, and `price_fix_duration` within the offer's active time window.
    /// Emits a `OfferMade` event upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `MakeOffer`.
    /// - `fee_basis_points`: Fee in basis points (e.g., 500 = 5%) charged when taking the offer.
    pub fn make_offer(ctx: Context<MakeOffer>, fee_basis_points: u64) -> Result<()> {
        offer::make_offer(ctx, fee_basis_points)
    }

    /// Closes a offer.
    ///
    /// Delegates to `offer::close_offer`.
    /// Removes the offer from the offers account and clears its data.
    /// Emits a `CloseOfferEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `CloseOffer`.
    /// - `offer_id`: ID of the offer to close.
    pub fn close_offer(ctx: Context<CloseOffer>, offer_id: u64) -> Result<()> {
        offer::close_offer(ctx, offer_id)
    }

    /// Adds a time vector to an existing offer.
    ///
    /// Delegates to `offer::add_offer_time_vector`.
    /// Creates a new time vector with auto-generated vector_id for the specified offer.
    /// Emits a `OfferVectorAdded` event upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `AddOfferVector`.
    /// - `offer_id`: ID of the offer to add the vector to.
    /// - `base_time`: Unix timestamp when the vector becomes active.
    /// - `base_price`: Price at the beginning of the vector.
    /// - `apr`: Annual Percentage Rate (APR) (see OfferVector::apr for details).
    /// - `price_fix_duration`: Duration in seconds for each price interval.
    pub fn add_offer_vector(
        ctx: Context<AddOfferVector>,
        offer_id: u64,
        base_time: u64,
        base_price: u64,
        apr: u64,
        price_fix_duration: u64,
    ) -> Result<()> {
        offer::add_offer_vector(
            ctx,
            offer_id,
            base_time,
            base_price,
            apr,
            price_fix_duration,
        )
    }

    /// Deletes a time vector from an offer.
    ///
    /// Delegates to `offer::delete_offer_vector`.
    /// Removes the specified time vector from the offer by setting it to default values.
    /// Only the boss can delete time vectors from offers.
    /// Emits a `OfferVectorDeleted` event upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `DeleteOfferVector`.
    /// - `offer_id`: ID of the offer containing the vector to delete.
    /// - `vector_id`: ID of the vector to delete.
    pub fn delete_offer_vector(
        ctx: Context<DeleteOfferVector>,
        offer_id: u64,
        vector_id: u64,
    ) -> Result<()> {
        offer::delete_offer_vector(ctx, offer_id, vector_id)
    }

    /// Updates the fee basis points for an offer.
    ///
    /// Delegates to `offer::update_offer_fee`.
    /// Allows the boss to modify the fee charged when users take the offer.
    /// Emits a `OfferFeeUpdatedEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `UpdateOfferFee`.
    /// - `offer_id`: ID of the offer to update.
    /// - `new_fee_basis_points`: New fee in basis points (0-10000).
    pub fn update_offer_fee(
        ctx: Context<UpdateOfferFee>,
        offer_id: u64,
        new_fee_basis_points: u64,
    ) -> Result<()> {
        offer::update_offer_fee(ctx, offer_id, new_fee_basis_points)
    }

    /// Takes a offer.
    ///
    /// Delegates to `offer::take_offer`.
    /// Allows a user to exchange token_in for token_out based on the offer's dynamic price.
    /// Emits a `TakeOfferEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TakeOffer`.
    /// - `offer_id`: ID of the offer to take.
    /// - `token_in_amount`: Amount of token_in to provide.
    pub fn take_offer(ctx: Context<TakeOffer>, offer_id: u64, token_in_amount: u64) -> Result<()> {
        offer::take_offer(ctx, offer_id, token_in_amount)
    }

    /// Takes a offer using permissionless flow with intermediary accounts.
    ///
    /// Delegates to `offer::take_offer_permissionless`.
    /// Similar to take_offer but routes token transfers through intermediary accounts
    /// owned by the program instead of direct user-to-boss and vault-to-user transfers.
    /// Emits a `TakeOfferPermissionlessEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TakeOfferPermissionless`.
    /// - `offer_id`: ID of the offer to take.
    /// - `token_in_amount`: Amount of token_in to provide.
    pub fn take_offer_permissionless(
        ctx: Context<TakeOfferPermissionless>,
        offer_id: u64,
        token_in_amount: u64,
    ) -> Result<()> {
        offer::take_offer_permissionless(ctx, offer_id, token_in_amount)
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
        initialize_permissionless::initialize_permissionless_account(ctx, name)
    }

    /// Updates the boss in the program state.
    ///
    /// Delegates to `set_boss::set_boss` to change the boss, emitting a `BossUpdated` event.
    pub fn set_boss(ctx: Context<SetBoss>, new_boss: Pubkey) -> Result<()> {
        state_operations::set_boss(ctx, new_boss)
    }

    /// Adds a new admin to the state.
    ///
    /// Delegates to `admin::add_admin` to add a new admin to the admin list.
    /// Only the boss can call this instruction to add new admins.
    /// # Arguments
    /// - `ctx`: Context for `AddAdmin`.
    /// - `new_admin`: Public key of the new admin to be added.
    pub fn add_admin(ctx: Context<AddAdmin>, new_admin: Pubkey) -> Result<()> {
        state_operations::add_admin(ctx, new_admin)
    }

    /// Removes an admin from the state.
    ///
    /// Delegates to `admin::remove_admin` to remove an admin from the admin list.
    /// Only the boss can call this instruction to remove admins.
    /// # Arguments
    /// - `ctx`: Context for `RemoveAdmin`.
    /// - `admin_to_remove`: Public key of the admin to be removed.
    pub fn remove_admin(ctx: Context<RemoveAdmin>, admin_to_remove: Pubkey) -> Result<()> {
        state_operations::remove_admin(ctx, admin_to_remove)
    }

    /// Clears all admins from the state.
    ///
    /// Delegates to `admin::clear_admins` to remove all admins from the admin list.
    /// Only the boss can call this instruction to clear all admins.
    pub fn clear_admins(ctx: Context<ClearAdmins>) -> Result<()> {
        state_operations::clear_admins(ctx)
    }

    /// Transfers mint authority from the boss to a program-derived PDA.
    ///
    /// Delegates to `mint_authority::transfer_mint_authority_to_program`.
    /// Only the boss can call this instruction to transfer mint authority for a specific token.
    /// The PDA is derived from the mint address and can later be used to mint tokens.
    /// Emits a `MintAuthorityTransferredToProgramEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TransferMintAuthorityToProgram`.
    pub fn transfer_mint_authority_to_program(
        ctx: Context<TransferMintAuthorityToProgram>,
    ) -> Result<()> {
        mint_authority::transfer_mint_authority_to_program(ctx)
    }

    /// Transfers mint authority from a program-derived PDA back to the boss.
    ///
    /// Delegates to `mint_authority::transfer_mint_authority_to_boss`.
    /// Only the boss can call this instruction to recover mint authority for a specific token.
    /// This serves as an emergency recovery mechanism.
    /// Emits a `MintAuthorityTransferredToBossEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TransferMintAuthorityToBoss`.
    pub fn transfer_mint_authority_to_boss(
        ctx: Context<TransferMintAuthorityToBoss>,
    ) -> Result<()> {
        mint_authority::transfer_mint_authority_to_boss(ctx)
    }

    /// Enables or disables the kill switch.
    ///
    /// Delegates to `kill_switch::kill_switch` to change the kill switch state.
    /// When enabled (true), the kill switch can halt critical program operations.
    /// When disabled (false), normal program operations can proceed.
    ///
    /// Access control:
    /// - Both boss and admins can enable the kill switch
    /// - Only the boss can disable the kill switch
    ///
    /// # Arguments
    /// - `ctx`: Context for `KillSwitch`.
    /// - `enable`: True to enable the kill switch, false to disable it.
    pub fn set_kill_switch(ctx: Context<SetKillSwitch>, enable: bool) -> Result<()> {
        state_operations::set_kill_switch(ctx, enable)
    }

    /// Sets the Onyc mint in the state.
    ///
    /// Delegates to `state_operations::set_onyc_mint` to change the Onyc mint.
    /// Only the boss can call this instruction to set the Onyc mint.
    /// Emits a `OnycMintSetEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `SetOnycMint`.
    pub fn set_onyc_mint(ctx: Context<SetOnycMint>) -> Result<()> {
        state_operations::set_onyc_mint(ctx)
    }

    /// Migrates the State account to include the new is_killed field.
    ///
    /// This instruction is required after deploying the updated program that includes
    /// the is_killed field in the State struct. It reallocates the account to the new size
    /// and initializes the kill switch to disabled (false) by default.
    ///
    /// # Security
    /// - Only the boss can perform this migration
    /// - The migration can only be performed once (subsequent calls will fail due to size constraints)
    ///
    /// # Arguments
    /// - `ctx`: Context for `MigrateState`.
    pub fn migrate_state(ctx: Context<MigrateState>) -> Result<()> {
        state_operations::migrate_state(ctx)
    }

    /// Gets the current NAV (price) for a specific offer.
    ///
    /// Delegates to `market_info::get_nav`.
    /// This is a read-only instruction that calculates and returns the current price
    /// for an offer based on its time vectors and APR parameters.
    /// Emits a `GetNAVEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `GetNAV`.
    /// - `offer_id`: ID of the offer to get the current price for.
    ///
    /// # Returns
    /// - `Ok(current_price)`: The calculated current price (mantissa) for the offer with scale=9
    pub fn get_nav(ctx: Context<GetNAV>, offer_id: u64) -> Result<u64> {
        market_info::get_nav(ctx, offer_id)
    }

    /// Gets the current APY (Annual Percentage Yield) for a specific offer.
    ///
    /// Delegates to `market_info::get_apy`.
    /// This is a read-only instruction that calculates and returns the current APY
    /// by converting the stored APR using daily compounding formula.
    /// Emits a `GetAPYEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `GetAPY`.
    /// - `offer_id`: ID of the offer to get the APY for.
    ///
    /// # Returns
    /// - `Ok(apy)`: The calculated APY scaled by 1_000_000 (returns the mantissa, with scale=6)
    pub fn get_apy(ctx: Context<GetAPY>, offer_id: u64) -> Result<u64> {
        market_info::get_apy(ctx, offer_id)
    }

    /// Gets the NAV adjustment (price change) for a specific offer.
    ///
    /// Delegates to `market_info::get_nav_adjustment`.
    /// This is a read-only instruction that calculates the price difference
    /// between the current vector and the previous vector at the current time.
    /// Returns a signed integer representing the price change.
    /// Emits a `GetNavAdjustmentEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `GetNavAdjustment`.
    /// - `offer_id`: ID of the offer to get the NAV adjustment for.
    ///
    /// # Returns
    /// - `Ok(adjustment)`: The calculated price adjustment (current - previous) as a signed integer,
    /// returns the mantissa with scale=9
    pub fn get_nav_adjustment(ctx: Context<GetNavAdjustment>, offer_id: u64) -> Result<i64> {
        market_info::get_nav_adjustment(ctx, offer_id)
    }

    /// Gets the current TVL (Total Value Locked) for a specific offer with 9 decimal precision
    ///
    /// Delegates to `market_info::get_tvl`.
    /// This is a read-only instruction that calculates and returns the current TVL
    /// for an offer based on the token_out supply and current NAV (price).
    /// TVL = token_out_supply * current_NAV
    /// Emits a `GetTVLEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `GetTVL`.
    /// - `offer_id`: ID of the offer to get the TVL for.
    ///
    /// # Returns
    /// - `Ok(tvl)`: The calculated TVL (mantissa) for the offer with scale=9
    pub fn get_tvl(ctx: Context<GetTVL>, offer_id: u64) -> Result<u64> {
        market_info::get_tvl(ctx, offer_id)
    }

    /// Delegates to `market_info::get_circulating_supply`.
    /// This is a read-only instruction that calculates and returns the current circulating supply
    /// for an offer based on the total token supply minus the vault amount.
    /// circulating_supply = total_supply - vault_amount
    /// Emits a `GetCirculatingSupplyEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `GetCirculatingSupply`.
    /// - `offer_id`: ID of the offer to get the circulating supply for.
    ///
    /// # Returns
    /// - `Ok(circulating_supply)`: The calculated circulating supply for the offer in base units
    pub fn get_circulating_supply(ctx: Context<GetCirculatingSupply>) -> Result<u64> {
        market_info::get_circulating_supply(ctx)
    }
}
