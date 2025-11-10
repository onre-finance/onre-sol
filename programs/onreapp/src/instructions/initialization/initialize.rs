use crate::constants::seeds;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::Mint;

/// Error codes for the initialize instruction
#[error_code]
pub enum InitializeErrorCode {
    /// Triggered when attempting to re-initialize a state that already has a boss set
    #[msg("Boss is already set, state has been initialized")]
    BossAlreadySet,
}

/// Account structure for initializing the program state
///
/// This struct defines the accounts required to set up the program's global state,
/// establishing the initial boss, ONyc mint, and default values for all state fields.
/// This is a one-time operation that creates the program's main state account.
///
/// # Preconditions
/// - The `state` account must not exist prior to execution; it will be created by this instruction
/// - The `boss` must have sufficient SOL to pay for account creation rent
/// - The `onyc_mint` must be a valid SPL Token mint account
///
/// # Postconditions
/// - Creates a new state account with the boss as the initial authority
/// - Sets all state fields to their default/initial values
/// - Enables normal program operations that depend on state
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The program state account to be created and initialized
    ///
    /// This account stores all global program configuration including:
    /// - Boss public key (program authority)
    /// - Kill switch state (initially disabled)
    /// - ONyc mint reference
    /// - Admin list (initially empty)
    /// - Approver for signature verification (initially unset)
    ///
    /// The account is created as a PDA derived from the "state" seed to ensure
    /// deterministic addressing and program ownership.
    #[account(
        init,
        payer = boss,
        space = 8 + State::INIT_SPACE,
        seeds = [seeds::STATE],
        bump
    )]
    pub state: Account<'info, State>,

    /// The initial boss who will have full authority over the program
    ///
    /// This signer becomes the program's boss and gains the ability to:
    /// - Create and manage offers
    /// - Update program state (boss, admins, kill switch, approver)
    /// - Perform administrative operations
    /// - Pay for the state account creation
    #[account(mut)]
    pub boss: Signer<'info>,

    /// The ONyc token mint that this program will manage
    ///
    /// This mint represents the protocol's native token and is used for:
    /// - Token minting operations when program has mint authority
    /// - Reference in various program calculations and operations
    pub onyc_mint: InterfaceAccount<'info, Mint>,

    /// Solana System program required for account creation and rent payment
    pub system_program: Program<'info, System>,
}

/// Initializes the program's global state account with default values
///
/// This function performs the one-time initialization of the program's main state account,
/// setting up all necessary fields with their initial values. This instruction must be
/// called before any other program operations can be performed.
///
/// # State Field Initialization
/// - `boss`: Set to the signer's public key (becomes program authority)
/// - `is_killed`: Set to false (normal operations enabled)
/// - `onyc_mint`: Set to the provided mint account
/// - `admins`: Array of default pubkeys (no admins initially)
/// - `approver`: Default pubkey (must be set separately via set_approver)
/// - `bump`: PDA bump seed for account validation
/// - `reserved`: Zero-filled bytes for future use
///
/// # Arguments
/// * `ctx` - Context containing the accounts needed for state initialization
///
/// # Returns
/// * `Ok(())` - If initialization completes successfully
/// * `Err(InitializeErrorCode::BossAlreadySet)` - If the state has already been initialized
///
/// # Security
/// - Only allows initialization if boss is currently unset (default pubkey)
pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    let state = &mut ctx.accounts.state;

    // Ensure this is the first initialization
    if state.boss != Pubkey::default() {
        return err!(InitializeErrorCode::BossAlreadySet);
    }

    // Set core state fields
    state.boss = ctx.accounts.boss.key();
    state.is_killed = false; // Normal operations enabled
    state.onyc_mint = ctx.accounts.onyc_mint.key();

    // Initialize admin list as empty
    state.admins = [Pubkey::default(); crate::state::MAX_ADMINS];

    // Leave approvers unset initially (must be configured via add_approver)
    state.approver1 = Pubkey::default();
    state.approver2 = Pubkey::default();

    // Store PDA bump for future validations
    state.bump = ctx.bumps.state;

    // Initialize max supply as 0 (no cap by default)
    state.max_supply = 0;

    // Initialize proposed_boss as unset
    state.proposed_boss = Pubkey::default();

    // Reserved space is automatically zero-initialized
    state.reserved = [0u8; 128];

    msg!(
        "Program state initialized: boss={}, onyc_mint={}, bump={}",
        state.boss,
        state.onyc_mint,
        state.bump
    );

    Ok(())
}
