use crate::constants::seeds;
use crate::instructions::buffer::accounts::{
    BufferAccrualAccountsBumps, __client_accounts_buffer_accrual_accounts,
    __cpi_client_accounts_buffer_accrual_accounts,
};
use crate::instructions::buffer::{
    accrue_buffer::{accrue_buffer_from_accounts, store_buffer_post_supply},
    BufferAccrualAccounts,
};
use crate::instructions::market_info::refresh_market_stats_pda;
use crate::instructions::offer::offer_utils::{
    is_onyc_token_out_mint, process_offer_core, should_accrue_onyc_mint, verify_offer_approval,
};
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
    #[msg("Invalid market stats PDA")]
    InvalidMarketStatsPda,
    #[msg("Market stats account must be writable")]
    MarketStatsNotWritable,
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
    /// CHECK: PDA derivation is validated by seeds constraint
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
    /// CHECK: PDA derivation is validated by seeds constraint
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

#[derive(Accounts)]
pub struct TakeOfferPermissionlessV2<'info> {
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

    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = state.is_killed == false @ TakeOfferPermissionlessErrorCode::KillSwitchActivated,
        has_one = boss @ TakeOfferPermissionlessErrorCode::InvalidBoss
    )]
    pub state: Box<Account<'info, State>>,

    /// CHECK: Account validation is enforced through state account has_one constraint
    pub boss: UncheckedAccount<'info>,
    /// CHECK: PDA derivation is validated by explicit key check in the handler
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_in_program
    )]
    pub vault_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_out_program
    )]
    pub vault_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated by explicit key check in the handler
    pub permissionless_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = permissionless_authority,
        associated_token::token_program = token_in_program
    )]
    pub permissionless_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = permissionless_authority,
        associated_token::token_program = token_out_program
    )]
    pub permissionless_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_in_program: Interface<'info, TokenInterface>,

    #[account(mut)]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_out_program: Interface<'info, TokenInterface>,

    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = user,
        associated_token::token_program = token_in_program
    )]
    pub user_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_out_mint,
        associated_token::authority = user,
        associated_token::token_program = token_out_program
    )]
    pub user_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_in_program
    )]
    pub boss_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA derivation is validated by explicit key check in the handler
    pub mint_authority: UncheckedAccount<'info>,
    pub buffer_accounts: BufferAccrualAccounts<'info>,

    /// CHECK: The handler validates PDA, writability, owner, and account data layout.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: Validated through address constraint to instructions sysvar
    #[account(address = sysvar::instructions::id())]
    pub instructions_sysvar: UncheckedAccount<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
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
#[inline(never)]
pub fn take_offer_permissionless(
    ctx: Context<TakeOfferPermissionless>,
    token_in_amount: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    execute_take_offer_permissionless(
        ctx.program_id,
        &ctx.accounts.offer,
        &ctx.accounts.state,
        &ctx.accounts.user,
        &ctx.accounts.instructions_sysvar,
        &ctx.accounts.token_in_mint,
        &mut ctx.accounts.token_out_mint,
        token_in_amount,
        &approval_message,
        &ctx.accounts.token_in_program,
        &ctx.accounts.user_token_in_account,
        &ctx.accounts.permissionless_token_in_account,
        &ctx.accounts.permissionless_authority,
        &ctx.accounts.boss_token_in_account,
        &ctx.accounts.vault_token_in_account,
        &ctx.accounts.vault_authority,
        &ctx.accounts.token_out_program,
        &ctx.accounts.vault_token_out_account,
        &ctx.accounts.permissionless_token_out_account,
        &ctx.accounts.user_token_out_account,
        &ctx.accounts.mint_authority,
        None,
        None,
        &ctx.accounts.system_program,
    )
}

#[inline(never)]
pub fn take_offer_permissionless_v2(
    ctx: Context<TakeOfferPermissionlessV2>,
    token_in_amount: u64,
    approval_message: Option<ApprovalMessage>,
) -> Result<()> {
    execute_take_offer_permissionless(
        ctx.program_id,
        &ctx.accounts.offer,
        &ctx.accounts.state,
        &ctx.accounts.user,
        &ctx.accounts.instructions_sysvar,
        &ctx.accounts.token_in_mint,
        &mut ctx.accounts.token_out_mint,
        token_in_amount,
        &approval_message,
        &ctx.accounts.token_in_program,
        &ctx.accounts.user_token_in_account,
        &ctx.accounts.permissionless_token_in_account,
        &ctx.accounts.permissionless_authority,
        &ctx.accounts.boss_token_in_account,
        &ctx.accounts.vault_token_in_account,
        &ctx.accounts.vault_authority,
        &ctx.accounts.token_out_program,
        &ctx.accounts.vault_token_out_account,
        &ctx.accounts.permissionless_token_out_account,
        &ctx.accounts.user_token_out_account,
        &ctx.accounts.mint_authority,
        Some(&ctx.accounts.buffer_accounts),
        Some(&ctx.accounts.market_stats),
        &ctx.accounts.system_program,
    )
}

fn execute_take_offer_permissionless<'info>(
    program_id: &Pubkey,
    offer_account: &AccountLoader<'info, Offer>,
    state: &Account<'info, State>,
    user: &Signer<'info>,
    instructions_sysvar: &UncheckedAccount<'info>,
    token_in_mint: &InterfaceAccount<'info, Mint>,
    token_out_mint: &mut InterfaceAccount<'info, Mint>,
    token_in_amount: u64,
    approval_message: &Option<ApprovalMessage>,
    token_in_program: &Interface<'info, TokenInterface>,
    user_token_in_account: &InterfaceAccount<'info, TokenAccount>,
    permissionless_token_in_account: &InterfaceAccount<'info, TokenAccount>,
    permissionless_authority: &UncheckedAccount<'info>,
    boss_token_in_account: &InterfaceAccount<'info, TokenAccount>,
    vault_token_in_account: &InterfaceAccount<'info, TokenAccount>,
    vault_authority: &UncheckedAccount<'info>,
    token_out_program: &Interface<'info, TokenInterface>,
    vault_token_out_account: &InterfaceAccount<'info, TokenAccount>,
    permissionless_token_out_account: &InterfaceAccount<'info, TokenAccount>,
    user_token_out_account: &InterfaceAccount<'info, TokenAccount>,
    mint_authority: &UncheckedAccount<'info>,
    buffer_accounts: Option<&BufferAccrualAccounts<'info>>,
    market_stats: Option<&UncheckedAccount<'info>>,
    system_program: &Program<'info, System>,
) -> Result<()> {
    let (va, va_bump) = Pubkey::find_program_address(&[seeds::OFFER_VAULT_AUTHORITY], program_id);
    require_keys_eq!(va, vault_authority.key());
    let (pa, pa_bump) =
        Pubkey::find_program_address(&[seeds::PERMISSIONLESS_AUTHORITY], program_id);
    require_keys_eq!(pa, permissionless_authority.key());
    let (ma, ma_bump) = Pubkey::find_program_address(&[seeds::MINT_AUTHORITY], program_id);
    require_keys_eq!(ma, mint_authority.key());

    let offer = offer_account.load()?;

    // Validate offer mints
    require_keys_eq!(
        offer.token_in_mint,
        token_in_mint.key(),
        OfferCoreError::InvalidTokenInMint
    );
    require_keys_eq!(
        offer.token_out_mint,
        token_out_mint.key(),
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
        approval_message,
        program_id,
        &user.key(),
        &state.approver1,
        &state.approver2,
        instructions_sysvar,
    )?;

    let result = process_offer_core(&offer, token_in_amount, token_in_mint, token_out_mint)?;
    let should_accrue = buffer_accounts
        .map(|accounts| {
            should_accrue_onyc_mint(
                state,
                token_out_mint,
                accounts.is_initialized(),
                &mint_authority.to_account_info(),
            )
        })
        .unwrap_or(false);
    let accrual = if should_accrue {
        let buffer_accounts = buffer_accounts.expect("checked above");
        Some(accrue_buffer_from_accounts(
            program_id,
            state,
            buffer_accounts,
            &offer,
            token_out_mint,
            mint_authority.to_account_info(),
            ma_bump,
            token_out_program,
        )?)
    } else {
        None
    };

    transfer_tokens(
        token_in_mint,
        token_in_program,
        user_token_in_account,
        permissionless_token_in_account,
        user,
        None,
        token_in_amount,
    )?;
    msg!("Transferred token_in from user to permissionless intermediary");

    execute_token_operations(ExecTokenOpsParams {
        token_in_program,
        token_in_mint,
        token_in_net_amount: result.token_in_net_amount,
        token_in_fee_amount: result.token_in_fee_amount,
        token_in_authority: &permissionless_authority.to_account_info(),
        token_in_source_signer_seeds: Some(&[&[seeds::PERMISSIONLESS_AUTHORITY, &[pa_bump]]]),
        vault_authority_signer_seeds: Some(&[&[seeds::OFFER_VAULT_AUTHORITY, &[va_bump]]]),
        token_in_source_account: permissionless_token_in_account,
        token_in_destination_account: boss_token_in_account,
        token_in_burn_account: vault_token_in_account,
        token_in_burn_authority: &vault_authority.to_account_info(),
        token_out_program,
        token_out_mint,
        token_out_amount: result.token_out_amount,
        token_out_authority: &vault_authority.to_account_info(),
        token_out_source_account: vault_token_out_account,
        token_out_destination_account: permissionless_token_out_account,
        mint_authority_pda: &mint_authority.to_account_info(),
        mint_authority_bump: &[ma_bump],
        token_out_max_supply: state.max_supply,
    })?;

    transfer_tokens(
        token_out_mint,
        token_out_program,
        permissionless_token_out_account,
        user_token_out_account,
        &permissionless_authority.to_account_info(),
        Some(&[&[seeds::PERMISSIONLESS_AUTHORITY, &[pa_bump]]]),
        result.token_out_amount,
    )?;

    if let Some(accrual) = accrual {
        let post_offer_supply = accrual
            .post_accrual_supply
            .checked_add(result.token_out_amount)
            .ok_or(OfferCoreError::OverflowError)?;
        store_buffer_post_supply(
            buffer_accounts.expect("accrual implies buffer accounts"),
            post_offer_supply,
            accrual.timestamp,
        )?;
    }

    if is_onyc_token_out_mint(state, token_out_mint) {
        if let Some(market_stats) = market_stats {
            let (market_stats_pda, _) =
                Pubkey::find_program_address(&[seeds::MARKET_STATS], program_id);
            require_keys_eq!(
                market_stats_pda,
                market_stats.key(),
                TakeOfferPermissionlessErrorCode::InvalidMarketStatsPda
            );
            require!(
                market_stats.is_writable,
                TakeOfferPermissionlessErrorCode::MarketStatsNotWritable
            );
            token_out_mint.reload()?;
            refresh_market_stats_pda(
                &offer,
                token_out_mint,
                &vault_token_out_account.to_account_info(),
                token_out_program,
                &market_stats.to_account_info(),
                &user.to_account_info(),
                &system_program.to_account_info(),
                program_id,
            )?;
        }
    }

    msg!(
        "Offer taken (permissionless) - PDA: {}, token_in(excluding fee): {}, fee: {}, token_out: {}, user: {}, price: {}",
        offer_account.key(),
        result.token_in_net_amount,
        result.token_in_fee_amount,
        result.token_out_amount,
        user.key,
        u64_to_dec9(result.current_price)
    );

    emit!(OfferTakenPermissionlessEvent {
        offer_pda: offer_account.key(),
        token_in_amount: result.token_in_net_amount,
        token_out_amount: result.token_out_amount,
        fee_amount: result.token_in_fee_amount,
        user: user.key(),
    });

    Ok(())
}
