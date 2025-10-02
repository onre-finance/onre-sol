use crate::constants::seeds;
use crate::instructions::offer::offer_utils::{process_offer_core, verify_offer_approval};
use crate::instructions::Offer;
use crate::state::{OfferVaultAuthority, PermissionlessAuthority, State};
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
pub struct TakeOfferPermissionlessEvent {
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
        bump = offer.load()?.bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// Program state account containing authorization and kill switch status
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ TakeOfferPermissionlessErrorCode::InvalidBoss,
        constraint = state.is_killed == false @ TakeOfferPermissionlessErrorCode::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// The boss account authorized to receive token_in payments
    ///
    /// Must match the boss stored in program state for security validation.
    /// CHECK: Account validation is enforced through state account constraint
    pub boss: UncheckedAccount<'info>,

    /// Program-derived authority that controls vault token operations
    ///
    /// This PDA manages token transfers and burning operations for the
    /// burn/mint architecture when program has mint authority.
    #[account(
        seeds = [seeds::OFFER_VAULT_AUTHORITY],
        bump = vault_authority.bump
    )]
    pub vault_authority: Account<'info, OfferVaultAuthority>,

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
    #[account(
        seeds = [seeds::PERMISSIONLESS_AUTHORITY], 
        bump = permissionless_authority.bump
    )]
    pub permissionless_authority: Account<'info, PermissionlessAuthority>,

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
    #[account(
        mut,
        constraint =
            token_in_mint.key() == offer.load()?.token_in_mint
            @ OfferCoreError::InvalidTokenInMint
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,
    
    /// Token program interface for input token operations
    pub token_in_program: Interface<'info, TokenInterface>,

    /// Output token mint account for the exchange
    ///
    /// Must be mutable to allow minting operations when program has mint authority.
    /// Validated against the offer's expected token_out_mint.
    #[account(
        mut,
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ OfferCoreError::InvalidTokenOutMint
    )]
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
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_in_program
    )]
    pub boss_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Program-derived mint authority for direct token minting
    ///
    /// Used when the program has mint authority and can mint token_out
    /// directly instead of transferring from vault.
    /// CHECK: PDA derivation is validated through seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        bump
    )]
    pub mint_authority_pda: UncheckedAccount<'info>,

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

/// Executes offers via permissionless flow with secure intermediary routing
///
/// This instruction enables users to execute offers without requiring direct token account
/// relationships with the boss by routing transfers through program-owned intermediary accounts.
/// This design supports permissionless access while maintaining security and atomicity.
///
/// The routing mechanism: User → Intermediary → Boss (token_in) and Vault/Mint → Intermediary → User (token_out)
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `token_in_amount` - Amount of token_in the user is willing to pay (including fees)
/// * `approval_message` - Optional cryptographic approval from trusted authority
///
/// # Process Flow
/// 1. Validate offer allows permissionless operations
/// 2. Verify approval requirements if offer needs approval
/// 3. Calculate current price and token amounts
/// 4. Execute atomic transfers through intermediary accounts
/// 5. Emit event with transaction details
///
/// # Returns
/// * `Ok(())` - If the offer is successfully executed
/// * `Err(PermissionlessNotAllowed)` - If offer doesn't allow permissionless operations
/// * `Err(_)` - If validation fails or token operations fail
///
/// # Access Control
/// - Only available for offers with allow_permissionless enabled
/// - Kill switch prevents execution when activated
/// - Approval verification when required
///
/// # Events
/// * `TakeOfferPermissionlessEvent` - Emitted with execution details and routing information
pub fn take_offer_permissionless(
    ctx: Context<TakeOfferPermissionless>,
    token_in_amount: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    let offer = ctx.accounts.offer.load()?;

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
        &ctx.accounts.state.approver,
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
        token_in_amount, // Including fee
        token_in_authority: &ctx.accounts.permissionless_authority.to_account_info(),
        token_in_source_signer_seeds: Some(&[&[
            seeds::PERMISSIONLESS_AUTHORITY,
            &[ctx.accounts.permissionless_authority.bump],
        ]]),
        vault_authority_signer_seeds: Some(&[&[
            seeds::OFFER_VAULT_AUTHORITY,
            &[ctx.accounts.vault_authority.bump],
        ]]),
        token_in_source_account: &ctx.accounts.permissionless_token_in_account,
        token_in_destination_account: &ctx.accounts.boss_token_in_account,
        token_in_burn_account: &ctx.accounts.vault_token_in_account,
        token_in_burn_authority: &ctx.accounts.vault_authority.to_account_info(),
        // Token out params
        token_out_program: &ctx.accounts.token_out_program,
        token_out_mint: &ctx.accounts.token_out_mint,
        token_out_amount: result.token_out_amount,
        token_out_authority: &ctx.accounts.vault_authority.to_account_info(),
        token_out_source_account: &ctx.accounts.vault_token_out_account,
        token_out_destination_account: &ctx.accounts.permissionless_token_out_account,
        mint_authority_pda: &ctx.accounts.mint_authority_pda,
        mint_authority_bump: &[ctx.bumps.mint_authority_pda],
    })?;

    transfer_tokens(
        &ctx.accounts.token_out_mint,
        &ctx.accounts.token_out_program,
        &ctx.accounts.permissionless_token_out_account,
        &ctx.accounts.user_token_out_account,
        &ctx.accounts.permissionless_authority.to_account_info(),
        Some(&[&[
            seeds::PERMISSIONLESS_AUTHORITY,
            &[ctx.accounts.permissionless_authority.bump],
        ]]),
        result.token_out_amount,
    )?;

    msg!(
        "Offer taken (permissionless) - PDA: {}, token_in(excluding fee): {}, fee: {}, token_out: {}, user: {}, price: {}",
        ctx.accounts.offer.key(),
        result.token_in_amount,
        result.fee_amount,
        result.token_out_amount,
        ctx.accounts.user.key,
        u64_to_dec9(result.current_price)
    );

    emit!(TakeOfferPermissionlessEvent {
        offer_pda: ctx.accounts.offer.key(),
        token_in_amount: result.token_in_amount,
        token_out_amount: result.token_out_amount,
        fee_amount: result.fee_amount,
        user: ctx.accounts.user.key(),
    });

    Ok(())
}
