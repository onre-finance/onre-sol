#![allow(unexpected_cfgs)]
#![allow(clippy::diverging_sub_expression)]

use anchor_lang::prelude::*;
use instructions::*;
use state::ConfigurableVaultKind;
use utils::ApprovalMessage;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;

pub use errors::OnreError;

const _ENV_FEATURE_COUNT: usize = cfg!(feature = "mainnet-test") as usize
    + cfg!(feature = "devnet-test") as usize
    + cfg!(feature = "devnet-dev") as usize;

const _: () = assert!(
    _ENV_FEATURE_COUNT < 2,
    "Environment features are mutually exclusive: enable at most one of 'mainnet-test', 'devnet-test', or 'devnet-dev'. Mainnet is the default when no environment feature is set."
);

// Program ID declaration
cfg_if::cfg_if! {
    if #[cfg(feature = "mainnet-test")] {
        mod program_id {
            anchor_lang::declare_id!("J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2");
        }
    } else if #[cfg(feature = "devnet-test")] {
        mod program_id {
            anchor_lang::declare_id!("J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2");
        }
    } else if #[cfg(feature = "devnet-dev")] {
        mod program_id {
            anchor_lang::declare_id!("devHfQHgiFNifkLW49RCXpyTUZMyKuBNnFSbrQ8XsbX");
        }
    } else {
        mod program_id {
            anchor_lang::declare_id!("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe");
        }
    }
}

pub use program_id::*;

pub use instructions::mint_authority::mint_to::MintTo;

/// The main program module for the Onre App.
///
/// This module defines the entry points for all program instructions. It facilitates
/// creation and management of offers where users provide `token_in` and receive
/// `token_out`. A key feature is the dynamic pricing model for offers, where the
/// `token_out` amount for a given `token_in` amount changes over time based on
/// configured pricing vectors.
///
/// Core functionalities include:
/// - Making offers with dynamic pricing (`make_offer`).
/// - Taking offers with current market pricing (`take_offer`, `take_offer_permissionless`).
/// - Managing offer vectors for price control (`add_offer_vector`, `delete_offer_vector`).
/// - Program state initialization and management (`initialize`, `propose_boss`, `accept_boss`, `add_admin`, `remove_admin`).
/// - Vault operations for token deposits and withdrawals (`offer_vault_deposit`, `offer_vault_withdraw`).
/// - Market information queries (`get_nav`, `get_apy`, `get_tvl`, `get_circulating_supply`).
/// - Mint authority management (`transfer_mint_authority_to_program`, `transfer_mint_authority_to_boss`).
/// - Emergency controls (`set_kill_switch`) and approval mechanisms (`add_approver`, `remove_approver`).
///
/// # Dynamic Pricing Model
/// The price for offers is determined by time-based vectors with APR (Annual Percentage Rate) growth:
/// - `start_time`: The timestamp when the vector becomes active.
/// - `base_time`: The timestamp used as the elapsed-time baseline for price growth.
/// - `base_price`: The price at `base_time` with 9 decimal precision.
/// - `apr`: Annual percentage rate with scale=6 (e.g., 10_000 = 1%, 1_000_000 = 100%).
/// - `price_fix_duration`: Duration in seconds for each discrete pricing step.
/// The price increases over time based on the APR, calculated in discrete intervals.
///
/// # Security
/// - Access controls are enforced, for example, ensuring only the `boss` can create offers or update critical state.
/// - PDA (Program Derived Address) accounts are used for offer and token authorities, ensuring ownership.
/// - Events are emitted for significant actions (e.g., `OfferMadeEvent`, `OfferTakenEvent`) for off-chain traceability.
#[program]
pub mod onreapp {
    use super::*;

    /// Initializes the program state and authority accounts.
    ///
    /// Delegates to `initialize::initialize` to set the initial boss in the state account.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::initialize(ctx)
    }

    /// Initializes a permissionless account.
    ///
    /// Delegates to `initialize::initialize_permissionless_authority` to create a new permissionless account.
    /// The account is created as a PDA with the seed "permissionless-1".
    /// Only the boss can initialize permissionless accounts.
    pub fn initialize_permissionless_authority(
        ctx: Context<InitializePermissionlessAuthority>,
        name: String,
    ) -> Result<()> {
        initialize_permissionless_authority::initialize_permissionless_authority(ctx, name)
    }

    /// Deposits tokens into the offer vault.
    ///
    /// Delegates to `vault_operations::offer_vault_deposit`.
    /// Transfers tokens from the depositor's token account to the offer vault token account for the specified mint.
    /// Creates vault token account if it doesn't exist using init_if_needed.
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

    /// Deposits tokens into the redemption vault.
    ///
    /// Delegates to `vault_operations::redemption_vault_deposit`.
    /// Transfers tokens from the depositor's token account to the redemption vault token account for the specified mint.
    /// Creates vault token account if it doesn't exist using init_if_needed.
    ///
    /// # Arguments
    /// - `ctx`: Context for `RedemptionVaultDeposit`.
    /// - `amount`: Amount of tokens to deposit.
    pub fn redemption_vault_deposit(
        ctx: Context<RedemptionVaultDeposit>,
        amount: u64,
    ) -> Result<()> {
        vault_operations::redemption_vault_deposit(ctx, amount)
    }

    /// Withdraws tokens from the redemption vault.
    ///
    /// Delegates to `vault_operations::redemption_vault_withdraw`.
    /// Transfers tokens from redemption vault's token account to boss's account for the specified mint.
    /// Creates boss token account if it doesn't exist using init_if_needed.
    /// Only the boss can call this instruction.
    ///
    /// # Arguments
    /// - `ctx`: Context for `RedemptionVaultWithdraw`.
    /// - `amount`: Amount of tokens to withdraw.
    pub fn redemption_vault_withdraw(
        ctx: Context<RedemptionVaultWithdraw>,
        amount: u64,
    ) -> Result<()> {
        vault_operations::redemption_vault_withdraw(ctx, amount)
    }

    pub fn set_configurable_vault_destination(
        ctx: Context<SetConfigurableVaultDestination>,
        kind: ConfigurableVaultKind,
        withdrawal_destination: Pubkey,
    ) -> Result<()> {
        configurable_vault::set_configurable_vault_destination(ctx, kind, withdrawal_destination)
    }

    pub fn withdraw_configurable_vault(
        ctx: Context<WithdrawConfigurableVault>,
        kind: ConfigurableVaultKind,
        amount: u64,
    ) -> Result<()> {
        configurable_vault::withdraw_configurable_vault(ctx, kind, amount)
    }

    /// Creates an offer.
    ///
    /// Delegates to `offer::make_offer`.
    /// Initializes the offer account for a token pair; pricing vectors are added separately.
    /// Emits an `OfferMadeEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `MakeOffer`.
    /// - `fee_basis_points`: Fee in basis points (e.g., 500 = 5%) charged when taking the offer.
    pub fn make_offer(
        ctx: Context<MakeOffer>,
        fee_basis_points: u16,
        needs_approval: bool,
        allow_permissionless: bool,
    ) -> Result<()> {
        offer::make_offer(ctx, fee_basis_points, needs_approval, allow_permissionless)
    }

    /// Adds a time vector to an existing offer.
    ///
    /// Delegates to `offer::add_offer_vector`.
    /// Creates a pricing vector for the specified offer. If `start_time` is not supplied,
    /// the vector starts at `max(base_time, current_time)`.
    /// Emits an `OfferVectorAddedEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `AddOfferVector`.
    /// - `start_time`: Unix timestamp when the vector becomes active.
    /// - `base_time`: Unix timestamp used as the elapsed-time baseline for price growth.
    /// - `base_price`: Price at the beginning of the vector.
    /// - `apr`: Annual Percentage Rate (APR) (see OfferVector::apr for details).
    /// - `price_fix_duration`: Duration in seconds for each price interval.
    pub fn add_offer_vector(
        ctx: Context<AddOfferVector>,
        start_time: Option<u64>,
        base_time: u64,
        base_price: u64,
        apr: u64,
        price_fix_duration: u64,
    ) -> Result<()> {
        offer::add_offer_vector(
            ctx,
            start_time,
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
    /// Emits an `OfferVectorDeletedEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `DeleteOfferVector`.
    /// - `vector_start_time`: Start time of the vector to delete.
    pub fn delete_offer_vector(
        ctx: Context<DeleteOfferVector>,
        vector_start_time: u64,
    ) -> Result<()> {
        offer::delete_offer_vector(ctx, vector_start_time)
    }

    /// Deletes all time vectors from an offer.
    ///
    /// Delegates to `offer::delete_all_offer_vectors`.
    /// Removes all time vectors from the offer regardless of their timing (past, active, or future).
    /// Only the boss can delete time vectors from offers.
    /// Emits a `AllOfferVectorsDeletedEvent` event upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `DeleteAllOfferVectors`.
    pub fn delete_all_offer_vectors(ctx: Context<DeleteAllOfferVectors>) -> Result<()> {
        offer::delete_all_offer_vectors(ctx)
    }

    /// Updates the fee basis points for an offer.
    ///
    /// Delegates to `offer::update_offer_fee`.
    /// Allows the boss to modify the fee charged when users take the offer.
    /// Emits a `OfferFeeUpdatedEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `UpdateOfferFee`.
    /// - `new_fee_basis_points`: New fee in basis points, capped at 1000 bps (10%).
    pub fn update_offer_fee(ctx: Context<UpdateOfferFee>, new_fee_basis_points: u16) -> Result<()> {
        offer::update_offer_fee(ctx, new_fee_basis_points)
    }

    /// Updates the fee basis points used only by permissionless offer execution.
    ///
    /// Delegates to `offer::update_offer_permissionless_fee`.
    pub fn update_offer_permissionless_fee(
        ctx: Context<UpdateOfferPermissionlessFee>,
        new_fee_basis_points_permissionless: u16,
    ) -> Result<()> {
        offer::update_offer_permissionless_fee(ctx, new_fee_basis_points_permissionless)
    }

    /// Updates the fee basis points used only by Prop AMM sell-side redemptions.
    pub fn update_redemption_offer_prop_amm_sell_fee(
        ctx: Context<UpdateRedemptionOfferFee>,
        new_fee_basis_points_prop_amm_sell: u16,
    ) -> Result<()> {
        redemption::update_redemption_offer_prop_amm_sell_fee(
            ctx,
            new_fee_basis_points_prop_amm_sell,
        )
    }

    /// Enables or disables one offer.
    ///
    /// Boss or admins can disable an offer. Only boss can re-enable it.
    pub fn set_offer_disabled(ctx: Context<SetOfferDisabled>, disabled: bool) -> Result<()> {
        offer::set_offer_disabled(ctx, disabled)
    }

    /// Takes an offer.
    ///
    /// Delegates to `offer::take_offer`.
    /// Allows a user to exchange token_in for token_out based on the offer's dynamic price.
    /// Emits an `OfferTakenEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TakeOffer`.
    /// - `token_in_amount`: Amount of token_in to provide.
    pub fn take_offer<'info>(
        ctx: Context<'info, TakeOffer<'info>>,
        token_in_amount: u64,
        approval_message: Option<ApprovalMessage>,
    ) -> Result<()> {
        offer::take_offer(ctx, token_in_amount, approval_message)
    }

    pub fn take_offer_v2<'info>(
        ctx: Context<'info, TakeOfferV2<'info>>,
        token_in_amount: u64,
        approval_message: Option<ApprovalMessage>,
    ) -> Result<()> {
        offer::take_offer_v2(ctx, token_in_amount, approval_message)
    }

    pub fn quote_swap_buy(ctx: Context<QuoteSwapBuy>, token_in_amount: u64) -> Result<()> {
        prop_amm::quote_swap_buy(ctx, token_in_amount)
    }

    pub fn quote_swap_sell(ctx: Context<QuoteSwapSell>, token_in_amount: u64) -> Result<()> {
        prop_amm::quote_swap_sell(ctx, token_in_amount)
    }

    pub fn open_swap_buy<'info>(
        ctx: Context<'info, OpenSwapBuy<'info>>,
        token_in_amount: u64,
        minimum_out: u64,
    ) -> Result<()> {
        prop_amm::open_swap_buy(ctx, token_in_amount, minimum_out)
    }

    pub fn open_swap_sell<'info>(
        ctx: Context<'info, OpenSwapSell<'info>>,
        token_in_amount: u64,
        minimum_out: u64,
    ) -> Result<()> {
        prop_amm::open_swap_sell(ctx, token_in_amount, minimum_out)
    }

    /// Takes an offer using permissionless flow with intermediary accounts.
    ///
    /// Delegates to `offer::take_offer_permissionless`.
    /// Similar to take_offer but routes token transfers through intermediary accounts
    /// owned by the program instead of direct user-to-boss and vault-to-user transfers.
    /// Permissionless execution does not require approval; the approval argument is
    /// retained only for legacy instruction-data compatibility.
    /// Emits an `OfferTakenPermissionlessEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `TakeOfferPermissionless`.
    /// - `token_in_amount`: Amount of token_in to provide.
    pub fn take_offer_permissionless<'info>(
        ctx: Context<'info, TakeOfferPermissionless<'info>>,
        token_in_amount: u64,
        approval_message: Option<ApprovalMessage>,
    ) -> Result<()> {
        offer::take_offer_permissionless(ctx, token_in_amount, approval_message)
    }

    /// Takes an offer through the V2 permissionless route without approval verification.
    ///
    /// Unlike the legacy permissionless instruction, V2 has no approval-message argument
    /// or instructions-sysvar account.
    pub fn take_offer_permissionless_v2<'info>(
        ctx: Context<'info, TakeOfferPermissionlessV2<'info>>,
        token_in_amount: u64,
    ) -> Result<()> {
        offer::take_offer_permissionless_v2(ctx, token_in_amount)
    }

    /// Proposes a new boss for ownership transfer.
    ///
    /// Delegates to `propose_boss::propose_boss` to propose a new boss authority.
    /// This is the first step in a two-step ownership transfer process.
    /// The proposed boss must then call accept_boss to complete the transfer.
    /// Emits a `BossProposedEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `ProposeBoss`.
    /// - `new_boss`: Public key of the proposed new boss.
    pub fn propose_boss(ctx: Context<ProposeBoss>, new_boss: Pubkey) -> Result<()> {
        state_operations::propose_boss(ctx, new_boss)
    }

    /// Accepts the proposed boss transfer.
    ///
    /// Delegates to `accept_boss::accept_boss` to complete the ownership transfer.
    /// This is the second step in a two-step ownership transfer process.
    /// Only the proposed boss can call this instruction to accept and become the new boss.
    /// Emits a `BossAcceptedEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `AcceptBoss`.
    pub fn accept_boss(ctx: Context<AcceptBoss>) -> Result<()> {
        state_operations::accept_boss(ctx)
    }

    /// Adds a new admin to the state.
    ///
    /// Delegates to `state_operations::add_admin` to add a new admin to the admin list.
    /// Only the boss can call this instruction to add new admins.
    /// # Arguments
    /// - `ctx`: Context for `AddAdmin`.
    /// - `new_admin`: Public key of the new admin to be added.
    pub fn add_admin(ctx: Context<AddAdmin>, new_admin: Pubkey) -> Result<()> {
        state_operations::add_admin(ctx, new_admin)
    }

    /// Removes an admin from the state.
    ///
    /// Delegates to `state_operations::remove_admin` to remove an admin from the admin list.
    /// Only the boss can call this instruction to remove admins.
    /// # Arguments
    /// - `ctx`: Context for `RemoveAdmin`.
    /// - `admin_to_remove`: Public key of the admin to be removed.
    pub fn remove_admin(ctx: Context<RemoveAdmin>, admin_to_remove: Pubkey) -> Result<()> {
        state_operations::remove_admin(ctx, admin_to_remove)
    }

    /// Clears all admins from the state.
    ///
    /// Delegates to `state_operations::clear_admins` to remove all admins from the admin list.
    /// Only the boss can call this instruction to clear all admins.
    pub fn clear_admins(ctx: Context<ClearAdmins>) -> Result<()> {
        state_operations::clear_admins(ctx)
    }

    /// Transfers mint authority from the boss to a program-derived PDA.
    ///
    /// Delegates to `mint_authority::transfer_mint_authority_to_program`.
    /// Only the boss can call this instruction to transfer mint authority for a specific token.
    /// The PDA is derived from the MINT_AUTHORITY seed and can later be used to mint tokens.
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
    /// Delegates to `state_operations::set_kill_switch` to change the kill switch state.
    /// When enabled (true), the kill switch halts guarded value-moving paths.
    /// When disabled (false), those guarded paths can proceed normally.
    ///
    /// Access control:
    /// - Both boss and admins can enable the kill switch
    /// - Only the boss can disable the kill switch
    ///
    /// # Arguments
    /// - `ctx`: Context for `SetKillSwitch`.
    /// - `enable`: True to enable the kill switch, false to disable it.
    pub fn set_kill_switch(ctx: Context<SetKillSwitch>, enable: bool) -> Result<()> {
        state_operations::set_kill_switch(ctx, enable)
    }

    /// Sets the Onyc mint in the state.
    ///
    /// Delegates to `state_operations::set_onyc_mint` to change the Onyc mint.
    /// Only the boss can call this instruction to set the Onyc mint.
    /// Emits an `ONycMintUpdatedEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `SetOnycMint`.
    pub fn set_onyc_mint(ctx: Context<SetOnycMint>) -> Result<()> {
        state_operations::set_onyc_mint(ctx)
    }

    /// Sets the worker authorized for redemption processing and BUFFER settlement.
    ///
    /// # Arguments
    /// - `ctx`: Context for `SetWorker`.
    /// - `new_worker`: Public key of the new worker.
    pub fn set_worker(ctx: Context<SetWorker>, new_worker: Pubkey) -> Result<()> {
        state_operations::set_worker(ctx, new_worker)
    }

    /// Initializes the standalone BUFFER pool state and vault accounts.
    ///
    /// Creates BUFFER state as a separate PDA so existing offer/redemption state
    /// remains unchanged. Only the boss can initialize BUFFER.
    pub fn initialize_buffer(ctx: Context<InitializeBuffer>) -> Result<()> {
        buffer::initialize_buffer(ctx)
    }

    /// Settles BUFFER accrual and refreshes market stats without a trade.
    /// Only the configured worker may call this instruction.
    pub fn settle_buffer(ctx: Context<SettleBuffer>) -> Result<()> {
        buffer::settle_buffer(ctx)
    }

    /// Sets the main offer stored in program state.
    ///
    /// Only the boss can update this value.
    pub fn set_main_offer(ctx: Context<SetMainOffer>) -> Result<()> {
        state_operations::set_main_offer(ctx)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn configure_prop_amm(
        ctx: Context<ConfigurePropAmm>,
        enabled: bool,
        curve_peg_haircut_bps: u16,
        curve_exponent_scaled: u32,
        cadence_threshold: u32,
        cadence_wave_scaled: u32,
        epoch_duration_seconds: i64,
        wall_sensitivity_scaled: u32,
        minimum_sell_haircut_onyc: u64,
    ) -> Result<()> {
        prop_amm::configure_prop_amm(
            ctx,
            enabled,
            curve_peg_haircut_bps,
            curve_exponent_scaled,
            cadence_threshold,
            cadence_wave_scaled,
            epoch_duration_seconds,
            wall_sensitivity_scaled,
            minimum_sell_haircut_onyc,
        )
    }

    /// Sets BUFFER gross yield.
    ///
    /// Accepts gross APR from 0% through 100% (scale = 1_000_000). Settles pending
    /// BUFFER accrual using the main offer, refreshes market stats, then updates gross APR.
    pub fn set_buffer_gross_apr(ctx: Context<SetBufferGrossYield>, gross_yield: u64) -> Result<()> {
        buffer::set_buffer_gross_apr(ctx, gross_yield)
    }

    /// Sets BUFFER fee split parameters.
    ///
    /// Both fee values are expressed in basis points and applied during accrual. When the
    /// performance-fee high-water mark is disabled, every nonzero accrual pays the configured
    /// performance fee regardless of current NAV.
    pub fn set_buffer_fee_config(
        ctx: Context<SetBufferFeeConfig>,
        management_fee_basis_points: u16,
        performance_fee_basis_points: u16,
        performance_fee_high_watermark_enabled: bool,
    ) -> Result<()> {
        buffer::set_buffer_fee_config(
            ctx,
            management_fee_basis_points,
            performance_fee_basis_points,
            performance_fee_high_watermark_enabled,
        )
    }

    /// Burns ONyc from the BUFFER reserve vault to preserve NAV after an asset-base adjustment.
    ///
    /// Callable by boss only.
    pub fn burn_for_nav_increase(
        ctx: Context<BurnForNavIncrease>,
        asset_adjustment_amount: u64,
    ) -> Result<()> {
        buffer::burn_for_nav_increase(ctx, asset_adjustment_amount)
    }

    /// Deposits ONyc into the BUFFER reserve vault.
    ///
    /// Callable by any signer.
    pub fn deposit_reserve_vault(ctx: Context<DepositReserveVault>, amount: u64) -> Result<()> {
        buffer::deposit_reserve_vault(ctx, amount)
    }

    /// Withdraws ONyc from the BUFFER reserve vault.
    ///
    /// Callable by boss only.
    pub fn withdraw_reserve_vault(ctx: Context<WithdrawReserveVault>, amount: u64) -> Result<()> {
        buffer::withdraw_reserve_vault(ctx, amount)
    }

    /// Mints ONyc tokens to the boss's account.
    ///
    /// Delegates to `mint_authority::mint_to` to mint ONyc tokens.
    /// Only the boss can call this instruction to mint ONyc tokens to their account.
    /// The program must have mint authority for the ONyc token.
    /// Emits an `OnycTokensMintedEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `MintTo`.
    /// - `amount`: Amount of ONyc tokens to mint.
    pub fn mint_to(ctx: Context<MintTo>, amount: u64) -> Result<()> {
        mint_authority::mint_to(ctx, amount)
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
    ///
    /// # Returns
    /// - `Ok(current_price)`: The calculated current price (mantissa) for the offer with scale=9
    pub fn get_nav(ctx: Context<GetNAV>) -> Result<u64> {
        market_info::get_nav(ctx)
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
    ///
    /// # Returns
    /// - `Ok(apy)`: The calculated APY scaled by 1_000_000 (returns the mantissa, with scale=6)
    pub fn get_apy(ctx: Context<GetAPY>) -> Result<u64> {
        market_info::get_apy(ctx)
    }

    /// Gets the NAV adjustment (price change) for a specific offer.
    ///
    /// Delegates to `market_info::get_nav_adjustment`.
    /// This is a read-only instruction that calculates the price jump at the
    /// active vector's start time by comparing the active vector's starting
    /// price with the previous vector's price at that same transition timestamp.
    /// Returns a signed integer representing the price change.
    /// Emits a `GetNavAdjustmentEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `GetNavAdjustment`.
    ///
    /// # Returns
    /// - `Ok(adjustment)`: The calculated price adjustment (current - previous) as a signed integer,
    /// returns the mantissa with scale=9
    pub fn get_nav_adjustment(ctx: Context<GetNavAdjustment>) -> Result<i64> {
        market_info::get_nav_adjustment(ctx)
    }

    /// Gets the current TVL (Total Value Locked) for an ONyc offer.
    ///
    /// Delegates to `market_info::get_tvl`.
    /// This legacy read-only instruction calculates TVL from circulating ONyc supply:
    /// total ONyc mint supply minus the offer-vault ONyc ATA balance.
    /// TVL = circulating_supply * current_NAV / 10^9.
    /// Emits a `GetTVLEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `GetTVL`.
    ///
    /// # Returns
    /// - `Ok(tvl)`: The calculated TVL in ONyc base units
    pub fn get_tvl(ctx: Context<GetTVL>) -> Result<u64> {
        market_info::get_tvl(ctx)
    }

    /// Delegates to `market_info::get_circulating_supply`.
    /// This is a read-only instruction that calculates and returns the current circulating supply
    /// based on total ONyc supply minus the offer-vault ONyc ATA balance.
    /// circulating_supply = total_supply - vault_amount
    /// Emits a `GetCirculatingSupplyEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `GetCirculatingSupply`.
    ///
    /// # Returns
    /// - `Ok(circulating_supply)`: The calculated global ONyc circulating supply in base units
    pub fn get_circulating_supply(ctx: Context<GetCirculatingSupply>) -> Result<u64> {
        market_info::get_circulating_supply(ctx)
    }

    /// Gets circulating supply using the cached excluded-balance PDA.
    pub fn get_circulating_supply_v2(ctx: Context<GetCirculatingSupplyV2>) -> Result<u64> {
        market_info::get_circulating_supply_v2(ctx)
    }

    /// Refreshes the canonical market-stats PDA using current on-chain state.
    ///
    /// Delegates to `market_info::refresh_market_stats`.
    /// Any signer can call this instruction and pay for PDA creation if needed, which
    /// allows backend automation to refresh market stats even on days without purchases.
    ///
    /// # Arguments
    /// - `ctx`: Context for `RefreshMarketStats`.
    pub fn refresh_market_stats(ctx: Context<RefreshMarketStats>) -> Result<()> {
        market_info::refresh_market_stats(ctx)
    }

    /// Gets ONyc TVL using the cached excluded-balance PDA.
    pub fn get_tvl_v2(ctx: Context<GetTVLV2>) -> Result<u64> {
        market_info::get_tvl_v2(ctx)
    }

    /// Replaces the owner list whose ONyc ATAs are excluded from circulating supply.
    /// Accepts at most 20 owners; an empty vector clears the list and unused stored slots are zeroed.
    pub fn set_circulating_supply_excluded_accounts(
        ctx: Context<SetCirculatingSupplyExcludedAccounts>,
        owners: Vec<Pubkey>,
    ) -> Result<()> {
        market_info::set_circulating_supply_excluded_accounts(ctx, owners)
    }

    /// Refreshes the cached excluded ONyc balance from the configured owner ATAs.
    pub fn update_circulating_supply_excluded_balance(
        ctx: Context<UpdateCirculatingSupplyExcludedBalance>,
    ) -> Result<()> {
        market_info::update_circulating_supply_excluded_balance(ctx)
    }

    /// Adds a trusted authority for approval verification.
    ///
    /// This instruction allows the boss to add an approver to one of the two available
    /// approver slots. If both slots are already filled, the instruction will fail.
    ///
    /// # Arguments
    /// - `ctx`: Context for `AddApprover`.
    /// - `approver`: Public key of the approver to add.
    pub fn add_approver(ctx: Context<AddApprover>, approver: Pubkey) -> Result<()> {
        state_operations::add_approver(ctx, approver)
    }

    /// Removes a trusted authority from approval verification.
    ///
    /// This instruction allows the boss to remove an approver by their public key.
    /// The approver must exist in either approver1 or approver2 slot, otherwise
    /// the instruction will fail.
    ///
    /// # Arguments
    /// - `ctx`: Context for `RemoveApprover`.
    /// - `approver`: Public key of the approver to remove.
    pub fn remove_approver(ctx: Context<RemoveApprover>, approver: Pubkey) -> Result<()> {
        state_operations::remove_approver(ctx, approver)
    }

    /// Configures the maximum supply cap for program-controlled minting paths.
    ///
    /// Delegates to `state_operations::configure_max_supply`.
    /// This instruction allows the boss to set or update the maximum supply cap
    /// that restricts program-controlled token minting. Setting to 0 removes the cap.
    /// Emits a `MaxSupplyConfiguredEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `ConfigureMaxSupply`.
    /// - `max_supply`: The maximum supply cap in base units (0 = no cap).
    pub fn configure_max_supply(ctx: Context<ConfigureMaxSupply>, max_supply: u64) -> Result<()> {
        state_operations::configure_max_supply(ctx, max_supply)
    }

    /// Configures the maximum amount allowed in one logical program mint operation.
    ///
    /// Setting to 0 removes the cap. BUFFER accrual checks the cap against the
    /// total gross accrual before splitting it across reserve and fee vaults.
    pub fn configure_max_mint_amount(
        ctx: Context<ConfigureMaxMintAmount>,
        max_mint_amount: u64,
    ) -> Result<()> {
        state_operations::configure_max_mint_amount(ctx, max_mint_amount)
    }

    /// Closes the program state account and returns the rent to the boss.
    ///
    /// Delegates to `state_operations::close_state`.
    /// This instruction closes the current main state account, clears its data,
    /// and transfers its rent balance back to the boss. The closed state data
    /// cannot be recovered; the state PDA can be initialized again later.
    /// Only the boss can call this instruction.
    /// Emits a `StateClosedEvent` upon success.
    ///
    /// # Warning
    /// This is a destructive operation for the current state configuration.
    /// Use with extreme caution.
    ///
    /// # Arguments
    /// - `ctx`: Context for `CloseState`.
    pub fn close_state(ctx: Context<CloseState>) -> Result<()> {
        state_operations::close_state(ctx)
    }

    /// Creates a redemption offer for converting output tokens from standard offers back
    /// to input tokens.
    ///
    /// Delegates to `redemption::make_redemption_offer`.
    /// This instruction initializes a new redemption offer that allows users to redeem
    /// token_out tokens from standard Offer (e.g. ONyc) for token_in tokens (e.g., USDC) at
    /// the current NAV price. The redemption offer is the inverse of the standard Offer.
    ///
    /// The redemption offer PDA is derived with reversed token order compared to the
    /// original offer, reflecting the inverse nature of the redemption operation.
    /// Emits a `RedemptionOfferCreatedEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `MakeRedemptionOffer`.
    /// - `fee_basis_points`: Fee in basis points, capped at 1000 bps (10%).
    /// - `fee_basis_points_prop_amm_sell`: Prop AMM sell fee in basis points, capped at 1000 bps (10%).
    ///
    /// # Access Control
    /// - Only the boss can call this instruction
    pub fn make_redemption_offer(
        ctx: Context<MakeRedemptionOffer>,
        fee_basis_points: u16,
        fee_basis_points_prop_amm_sell: u16,
    ) -> Result<()> {
        redemption::make_redemption_offer(ctx, fee_basis_points, fee_basis_points_prop_amm_sell)
    }

    pub fn set_redemption_offer_disabled(
        ctx: Context<SetRedemptionOfferDisabled>,
        disabled: bool,
    ) -> Result<()> {
        redemption::set_redemption_offer_disabled(ctx, disabled)
    }

    /// Creates a redemption request.
    ///
    /// Delegates to `redemption::create_redemption_request`.
    /// This instruction creates a new redemption request that allows users to request
    /// redemption of token_in tokens for token_out tokens at a future time. Anyone can
    /// create a redemption request by paying for the PDA rent.
    /// Emits a `RedemptionRequestCreatedEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `CreateRedemptionRequest`.
    /// - `amount`: Amount of token_in tokens to redeem.
    /// - `request_id`: Frontend-generated ID used with the offer and redeemer to derive the PDA.
    pub fn create_redemption_request(
        ctx: Context<CreateRedemptionRequest>,
        amount: u64,
        request_id: String,
    ) -> Result<()> {
        redemption::create_redemption_request(ctx, amount, request_id)
    }

    /// Fulfills a redemption request with ONyc buffer accrual support.
    ///
    /// Delegates to `redemption::fulfill_redemption_request`.
    pub fn fulfill_redemption_request<'info>(
        ctx: Context<'info, FulfillRedemptionRequest<'info>>,
        amount: u64,
    ) -> Result<()> {
        redemption::fulfill_redemption_request(ctx, amount)
    }

    /// Cancels a redemption request.
    ///
    /// Delegates to `redemption::cancel_redemption_request`.
    /// This instruction cancels an unfulfilled or partially fulfilled redemption request.
    /// The request can be cancelled by the redeemer, worker, or boss. Upon
    /// cancellation, the unfulfilled token_in amount is returned to the redeemer, that
    /// returned amount is subtracted from the redemption offer's requested_redemptions,
    /// and the redemption request account is closed.
    /// Emits a `RedemptionRequestCancelledEvent` upon success.
    ///
    /// # Arguments
    /// - `ctx`: Context for `CancelRedemptionRequest`.
    ///
    /// # Access Control
    /// - Signer must be one of: redeemer, worker, or boss
    /// - Request must have an unfulfilled amount remaining.
    pub fn cancel_redemption_request<'info>(
        ctx: Context<'info, CancelRedemptionRequest<'info>>,
    ) -> Result<()> {
        redemption::cancel_redemption_request(ctx)
    }

    /// Updates the fee configuration for a specific redemption offer.
    ///
    /// This instruction allows the boss to modify the fee charged when fulfilling
    /// redemption requests for a specific redemption offer. Only the boss can call this instruction.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context
    /// * `new_fee_basis_points` - New fee in basis points, capped at 1000 bps (10%).
    ///
    /// # Access Control
    /// - Boss only
    pub fn update_redemption_offer_fee(
        ctx: Context<UpdateRedemptionOfferFee>,
        new_fee_basis_points: u16,
    ) -> Result<()> {
        redemption::update_redemption_offer_fee(ctx, new_fee_basis_points)
    }

    pub fn update_redemption_offer_vault_target(
        ctx: Context<UpdateRedemptionOfferFee>,
        new_vault_target_bps: u16,
    ) -> Result<()> {
        redemption::update_redemption_offer_vault_target(ctx, new_vault_target_bps)
    }
}
