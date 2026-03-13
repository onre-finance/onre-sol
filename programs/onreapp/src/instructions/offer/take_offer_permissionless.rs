use crate::constants::seeds;
use crate::instructions::fee_config::{FeeConfig, FeeConfigError, FeeType};
use crate::instructions::offer::offer_utils::{process_offer_core, verify_offer_approval};
use crate::instructions::Offer;
use crate::state::State;
use crate::utils::{
    execute_token_operations, transfer_tokens, u64_to_dec9, ApprovalMessage, ExecTokenOpsParams,
};
use crate::OfferCoreError;
use anchor_lang::{prelude::*, solana_program::sysvar, Accounts};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Error codes specific to the take_offer_permissionless instruction
#[error_code]
pub enum TakeOfferPermissionlessErrorCode {
    /// The boss account does not match the one stored in program state
    #[msg("Invalid boss account")]
    InvalidBoss,
    /// The program kill switch is activated, preventing offer operations
    #[msg("Kill switch is activated")]
    KillSwitchActivated,
    /// The offer does not allow permissionless operations
    #[msg("Permissionless take offer not allowed")]
    PermissionlessNotAllowed,
}

/// Event emitted when an offer is successfully executed via permissionless flow
///
/// Provides transparency for tracking permissionless offer execution with intermediary routing.
#[event]
pub struct OfferTakenPermissionlessEvent {
    /// The PDA address of the offer that was executed
    pub offer_pda: Pubkey,
    /// Amount of token_in paid by the user after fee deduction
    pub token_in_amount: u64,
    /// Amount of token_out received by the user
    pub token_out_amount: u64,
    /// Fee amount deducted from the original token_in payment
    pub fee_amount: u64,
    /// Public key of the user who executed the offer
    pub user: Pubkey,
}

/// Account structure for executing offers via permissionless flow with intermediary routing
///
/// This struct defines all accounts required for permissionless offer execution including
/// program-owned intermediary accounts that enable secure token routing without requiring
/// direct user-to-boss token account relationships.
#[derive(Accounts)]
pub struct TakeOfferPermissionless<'info> {
    /// The offer account containing pricing vectors and configuration
    ///
    /// Must have allow_permissionless enabled for this instruction to succeed.
    /// Contains pricing vectors for dynamic price calculation.
    #[account(
        mut,
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// Program state account containing authorization and kill switch status
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = state.is_killed == false @ TakeOfferPermissionlessErrorCode::KillSwitchActivated,
        has_one = boss @ TakeOfferPermissionlessErrorCode::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    /// The boss account authorized to receive token_in payments
    ///
    /// Must match the boss stored in program state for security validation.
    /// CHECK: Account validation is enforced through state account has_one constraint
    pub boss: UncheckedAccount<'info>,

    /// Program-derived authority that controls vault token operations
    ///
    /// This PDA manages token transfers and burning operations for the
    /// burn/mint architecture when program has mint authority.
    /// CHECK: PDA derivation is validated at runtime in validate_permissionless_pdas
    pub vault_authority: UncheckedAccount<'info>,

    /// Vault account for temporary token_in storage during burn operations
    ///
    /// Used for burning input tokens when the program has mint authority
    /// for efficient burn/mint token exchange architecture.
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_in_program
    )]
    pub vault_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Vault account for token_out distribution when using transfer mechanism
    ///
    /// Source of output tokens when the program lacks mint authority
    /// and must transfer from pre-funded vault instead of minting.
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_out_program
    )]
    pub vault_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Program-derived authority that controls intermediary token routing accounts
    ///
    /// This PDA manages the intermediary accounts used for permissionless token
    /// routing, enabling secure transfers without direct user-boss relationships.
    /// CHECK: PDA derivation is validated at runtime in validate_permissionless_pdas
    pub permissionless_authority: UncheckedAccount<'info>,

    /// Intermediary account for routing token_in payments
    ///
    /// Temporary holding account that receives user payments before forwarding
    /// to boss, enabling permissionless operations without direct relationships.
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = permissionless_authority,
        associated_token::token_program = token_in_program
    )]
    pub permissionless_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Intermediary account for routing token_out distributions
    ///
    /// Temporary holding account that receives output tokens before forwarding
    /// to user, completing the permissionless routing mechanism.
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = permissionless_authority,
        associated_token::token_program = token_out_program
    )]
    pub permissionless_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Input token mint account for the exchange
    ///
    /// Must be mutable to allow burning operations when program has mint authority.
    /// Validated against the offer's expected token_in_mint.
    #[account(mut)]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for input token operations
    pub token_in_program: Interface<'info, TokenInterface>,

    /// Output token mint account for the exchange
    ///
    /// Must be mutable to allow minting operations when program has mint authority.
    /// Validated against the offer's expected token_out_mint.
    #[account(mut)]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for output token operations
    pub token_out_program: Interface<'info, TokenInterface>,

    /// User's input token account for payment
    ///
    /// Source account from which the user pays token_in for the exchange.
    /// Must have sufficient balance for the requested token_in_amount.
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = user,
        associated_token::token_program = token_in_program
    )]
    pub user_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User's output token account for receiving exchanged tokens
    ///
    /// Destination account where the user receives token_out from the exchange.
    /// Created automatically if it doesn't exist using init_if_needed.
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_out_mint,
        associated_token::authority = user,
        associated_token::token_program = token_out_program
    )]
    pub user_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Boss's input token account for receiving payments
    ///
    /// Final destination account where the boss receives token_in payments
    /// from users taking offers via intermediary routing.
    /// Validated at runtime (owner + mint) to avoid an extra find_program_address
    /// in try_accounts which would overflow the stack.
    #[account(mut)]
    pub boss_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Fee config PDA for TakeOffer fee routing
    #[account(
        seeds = [seeds::FEE_CONFIG, &[FeeType::TakeOffer as u8]],
        bump = fee_config.bump
    )]
    pub fee_config: Box<Account<'info, FeeConfig>>,

    /// The owner of the fee destination ATA (fee_config.destination or fee_config PDA)
    /// CHECK: validated at runtime in validate_permissionless_pdas
    #[account(mut)]
    pub fee_destination_owner: UncheckedAccount<'info>,

    /// Fee destination token account - created if needed (payer = user)
    /// CHECK: address validated implicitly via create_idempotent CPI
    #[account(mut)]
    pub fee_destination_token_account: UncheckedAccount<'info>,

    /// Program-derived mint authority for direct token minting
    ///
    /// Used when the program has mint authority and can mint token_out
    /// directly instead of transferring from vault.
    /// CHECK: PDA derivation is validated at runtime in validate_permissionless_pdas
    pub mint_authority: UncheckedAccount<'info>,

    /// Instructions sysvar for approval signature verification
    ///
    /// Required for cryptographic verification of approval messages
    /// when offers require boss approval for execution.
    /// CHECK: Validated through address constraint to instructions sysvar
    #[account(address = sysvar::instructions::id())]
    pub instructions_sysvar: UncheckedAccount<'info>,

    /// The user executing the offer and paying for account creation
    #[account(mut)]
    pub user: Signer<'info>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program required for account creation
    pub system_program: Program<'info, System>,
}

/// Validates PDA accounts and fee destination, returns bumps for signer seeds.
///
/// Extracted into a separate `#[inline(never)]` function to isolate the stack-heavy
/// `find_program_address` calls (3 total) from the main handler. Each call uses
/// SHA256 hashing which requires significant stack space. Combined with the large
/// accounts struct, keeping these in the main handler would exceed the 4096-byte
/// BPF stack frame limit.
#[inline(never)]
fn validate_permissionless_pdas(
    vault_authority_key: &Pubkey,
    permissionless_authority_key: &Pubkey,
    mint_authority_key: &Pubkey,
    fee_config: &FeeConfig,
    fee_config_key: &Pubkey,
    fee_destination_owner_key: &Pubkey,
    boss_token_in_owner: &Pubkey,
    boss_token_in_mint: &Pubkey,
    boss_key: &Pubkey,
    token_in_mint_key: &Pubkey,
    program_id: &Pubkey,
) -> Result<(u8, u8, u8)> {
    let (va, va_bump) =
        Pubkey::find_program_address(&[seeds::OFFER_VAULT_AUTHORITY], program_id);
    require_keys_eq!(va, *vault_authority_key);
    let (pa, pa_bump) =
        Pubkey::find_program_address(&[seeds::PERMISSIONLESS_AUTHORITY], program_id);
    require_keys_eq!(pa, *permissionless_authority_key);
    let (ma, ma_bump) = Pubkey::find_program_address(&[seeds::MINT_AUTHORITY], program_id);
    require_keys_eq!(ma, *mint_authority_key);

    // Validate boss_token_in_account (moved from associated_token constraint to reduce
    // try_accounts stack size)
    require_keys_eq!(
        *boss_token_in_owner,
        *boss_key,
        OfferCoreError::InvalidTokenInMint
    );
    require_keys_eq!(
        *boss_token_in_mint,
        *token_in_mint_key,
        OfferCoreError::InvalidTokenInMint
    );

    // Validate fee destination owner
    let expected_fee_owner = fee_config.fee_destination_owner(fee_config_key);
    require_keys_eq!(
        *fee_destination_owner_key,
        expected_fee_owner,
        FeeConfigError::InvalidFeeDestination
    );

    Ok((va_bump, pa_bump, ma_bump))
}

/// Executes offers via permissionless flow with secure intermediary routing
///
/// This instruction enables users to execute offers without requiring direct token account
/// relationships with the boss by routing transfers through program-owned intermediary accounts.
/// This design supports permissionless access while maintaining security and atomicity.
///
/// The routing mechanism: User -> Intermediary -> Boss (token_in) and Vault/Mint -> Intermediary -> User (token_out)
#[inline(never)]
pub fn take_offer_permissionless(
    ctx: Context<TakeOfferPermissionless>,
    token_in_amount: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let (va_bump, pa_bump, ma_bump) = validate_permissionless_pdas(
        &ctx.accounts.vault_authority.key(),
        &ctx.accounts.permissionless_authority.key(),
        &ctx.accounts.mint_authority.key(),
        &ctx.accounts.fee_config,
        &ctx.accounts.fee_config.key(),
        &ctx.accounts.fee_destination_owner.key(),
        &ctx.accounts.boss_token_in_account.owner,
        &ctx.accounts.boss_token_in_account.mint,
        &ctx.accounts.boss.key(),
        &ctx.accounts.token_in_mint.key(),
        ctx.program_id,
    )?;

    // Idempotent ATA creation: user pays rent if not yet initialized
    anchor_spl::associated_token::create_idempotent(CpiContext::new(
        ctx.accounts.associated_token_program.to_account_info(),
        anchor_spl::associated_token::Create {
            payer: ctx.accounts.user.to_account_info(),
            associated_token: ctx.accounts.fee_destination_token_account.to_account_info(),
            authority: ctx.accounts.fee_destination_owner.to_account_info(),
            mint: ctx.accounts.token_in_mint.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            token_program: ctx.accounts.token_in_program.to_account_info(),
        },
    ))?;

    let offer = ctx.accounts.offer.load()?;

    // Validate offer mints
    require_keys_eq!(
        offer.token_in_mint,
        ctx.accounts.token_in_mint.key(),
        OfferCoreError::InvalidTokenInMint
    );
    require_keys_eq!(
        offer.token_out_mint,
        ctx.accounts.token_out_mint.key(),
        OfferCoreError::InvalidTokenOutMint
    );
    // Validate if offer allows permissionless access
    require!(
        offer.allow_permissionless(),
        TakeOfferPermissionlessErrorCode::PermissionlessNotAllowed
    );

    // Verify approval if needed
    verify_offer_approval(
        &offer,
        &approval_message,
        ctx.program_id,
        &ctx.accounts.user.key(),
        &ctx.accounts.state.approver1,
        &ctx.accounts.state.approver2,
        &ctx.accounts.instructions_sysvar,
    )?;

    // Use shared core processing logic
    let result = process_offer_core(
        &offer,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
    )?;

    // 1. Transfer token_in from user to permissionless intermediary
    transfer_tokens(
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_in_program,
        &ctx.accounts.user_token_in_account,
        &ctx.accounts.permissionless_token_in_account,
        &ctx.accounts.user,
        None,
        token_in_amount,
    )?;
    msg!("Transferred token_in from user to permissionless intermediary");

    // 2. Execute token operations (transfer + burn for token_in, transfer for token_out)
    execute_token_operations(ExecTokenOpsParams {
        // Token in params
        token_in_program: &ctx.accounts.token_in_program,
        token_in_mint: &ctx.accounts.token_in_mint,
        token_in_net_amount: result.token_in_net_amount,
        token_in_fee_amount: result.token_in_fee_amount,
        token_in_authority: &ctx.accounts.permissionless_authority.to_account_info(),
        token_in_source_signer_seeds: Some(&[&[seeds::PERMISSIONLESS_AUTHORITY, &[pa_bump]]]),
        vault_authority_signer_seeds: Some(&[&[seeds::OFFER_VAULT_AUTHORITY, &[va_bump]]]),
        token_in_source_account: &ctx.accounts.permissionless_token_in_account,
        token_in_fee_destination_account: &ctx.accounts.fee_destination_token_account.to_account_info(),
        token_in_boss_account: &ctx.accounts.boss_token_in_account,
        token_in_burn_account: &ctx.accounts.vault_token_in_account,
        token_in_burn_authority: &ctx.accounts.vault_authority.to_account_info(),
        // Token out params
        token_out_program: &ctx.accounts.token_out_program,
        token_out_mint: &ctx.accounts.token_out_mint,
        token_out_amount: result.token_out_amount,
        token_out_authority: &ctx.accounts.vault_authority.to_account_info(),
        token_out_source_account: &ctx.accounts.vault_token_out_account,
        token_out_destination_account: &ctx.accounts.permissionless_token_out_account,
        mint_authority_pda: &ctx.accounts.mint_authority.to_account_info(),
        mint_authority_bump: &[ma_bump],
        token_out_max_supply: ctx.accounts.state.max_supply,
    })?;

    transfer_tokens(
        &ctx.accounts.token_out_mint,
        &ctx.accounts.token_out_program,
        &ctx.accounts.permissionless_token_out_account,
        &ctx.accounts.user_token_out_account,
        &ctx.accounts.permissionless_authority.to_account_info(),
        Some(&[&[seeds::PERMISSIONLESS_AUTHORITY, &[pa_bump]]]),
        result.token_out_amount,
    )?;

    msg!(
        "Offer taken (permissionless) - PDA: {}, token_in(excluding fee): {}, fee: {}, token_out: {}, user: {}, price: {}",
        ctx.accounts.offer.key(),
        result.token_in_net_amount,
        result.token_in_fee_amount,
        result.token_out_amount,
        ctx.accounts.user.key,
        u64_to_dec9(result.current_price)
    );

    emit!(OfferTakenPermissionlessEvent {
        offer_pda: ctx.accounts.offer.key(),
        token_in_amount: result.token_in_net_amount,
        token_out_amount: result.token_out_amount,
        fee_amount: result.token_in_fee_amount,
        user: ctx.accounts.user.key(),
    });

    Ok(())
}
