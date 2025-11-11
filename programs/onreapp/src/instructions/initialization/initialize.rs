use crate::constants::{seeds, MAX_ADMINS};
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::bpf_loader;
use anchor_lang::solana_program::bpf_loader_upgradeable::{
    self, get_program_data_address, UpgradeableLoaderState,
};
use anchor_lang::Accounts;
use anchor_spl::token_interface::Mint;

/// Error codes for the initialize instruction
#[error_code]
pub enum InitializeErrorCode {
    /// Triggered when attempting to re-initialize a state that already has a boss set
    #[msg("Boss is already set, state has been initialized")]
    BossAlreadySet,

    #[msg("Signer does not match the program's upgrade authority")]
    WrongBoss,

    #[msg("Wrong owner")]
    WrongOwner,

    #[msg("Program has no upgrade authority")]
    ImmutableProgram,

    #[msg("Wrong program data")]
    WrongProgramData,

    #[msg("Program data account not provided")]
    MissingProgramData,

    #[msg("Failed to deserialize program data")]
    DeserializeProgramDataFailed,

    #[msg("Account is not ProgramData")]
    NotProgramData,
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

    /// The offer mint authority account to initialize, rent paid by `boss`.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        init_if_needed,
        payer = boss,
        space = 8,
        seeds = [seeds::MINT_AUTHORITY],
        bump
    )]
    pub mint_authority: AccountInfo<'info>,

    /// The offer vault authority account to initialize, rent paid by `boss`.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        init_if_needed,
        payer = boss,
        space = 8,
        seeds = [seeds::OFFER_VAULT_AUTHORITY],
        bump
    )]
    pub offer_vault_authority: AccountInfo<'info>,

    /// The initial boss who will have full authority over the program
    ///
    /// This signer becomes the program's boss and gains the ability to:
    /// - Create and manage offers
    /// - Update program state (boss, admins, kill switch, approver)
    /// - Perform administrative operations
    /// - Pay for the state account creation
    #[account(mut)]
    pub boss: Signer<'info>,

    /// CHECK: This must be *this* program's executable account
    #[account(executable, address = crate::ID)]
    pub program: AccountInfo<'info>,

    /// CHECK: ProgramData PDA for `program` under the upgradeable loader
    /// We'll verify its address in code.
    pub program_data: Option<AccountInfo<'info>>,

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
    let upgrade_authority =
        get_upgrade_authority(&ctx.accounts.program, ctx.accounts.program_data.as_ref())?;

    if upgrade_authority.is_some() {
        // Check that the boss is the upgrade authority
        require_keys_eq!(
            ctx.accounts.boss.key(),
            upgrade_authority.unwrap(),
            InitializeErrorCode::WrongOwner
        );
    }

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
    state.admins = [Pubkey::default(); MAX_ADMINS];

    // Leave approvers unset initially (must be configured via add_approver)
    state.approver1 = Pubkey::default();
    state.approver2 = Pubkey::default();

    // Store PDA bump for future validations
    state.bump = ctx.bumps.state;

    // Initialize max supply as 0 (no cap by default)
    state.max_supply = 0;

    // Initialize proposed_boss as unset
    state.proposed_boss = Pubkey::default();

    msg!(
        "Program state initialized: boss={}, onyc_mint={}, bump={}",
        state.boss,
        state.onyc_mint,
        state.bump
    );

    Ok(())
}

/// Returns the Option<Pubkey> of the upgrade authority for an upgradeable program.
///
/// Required accounts:
/// - `program`: the *executable* program AccountInfo (must equal crate::ID)
/// - `program_data`: the ProgramData account for `program`
pub fn get_upgrade_authority(
    program: &AccountInfo,
    program_data: Option<&AccountInfo>,
) -> Result<Option<Pubkey>> {
    let owner = program.owner;

    if owner == &bpf_loader_upgradeable::id() {
        let program_data =
            program_data.ok_or_else(|| error!(InitializeErrorCode::MissingProgramData))?;
        require!(
            program_data.owner == &bpf_loader_upgradeable::id(),
            InitializeErrorCode::WrongOwner
        );

        // Ensure the ProgramData really belongs to this program
        let expected_pd = get_program_data_address(program.key);
        require_keys_eq!(
            expected_pd,
            *program_data.key,
            InitializeErrorCode::WrongProgramData
        );

        // Read ProgramData and extract the authority
        let data = program_data
            .try_borrow_data()
            .map_err(|_| error!(InitializeErrorCode::DeserializeProgramDataFailed))?;
        // Newer Solana crates provide `deserialize`; if not, switch to bincode.
        let state = bincode::deserialize(&data).map_err(|_| ProgramError::InvalidAccountData)?;

        if let UpgradeableLoaderState::ProgramData {
            upgrade_authority_address,
            ..
        } = state
        {
            Ok(upgrade_authority_address) // Some(pubkey) or None
        } else {
            err!(InitializeErrorCode::NotProgramData)
        }
    } else if owner == &bpf_loader::id() {
        // Required for tests to work. For BPF_LOADER, there is no upgrade_authority so we can't actually check it
        Ok(None)
    } else {
        err!(InitializeErrorCode::WrongOwner)
    }
}
