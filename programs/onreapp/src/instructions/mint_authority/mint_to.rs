use crate::constants::seeds;
use crate::instructions::buffer::{
    __client_accounts_buffer_accrual_accounts, __cpi_client_accounts_buffer_accrual_accounts,
    accounts::BufferAccrualAccountsBumps,
    accrue_buffer::{accrue_buffer_from_accounts, store_buffer_post_supply},
    modular_accrual::{self, BufferContext},
    BufferAccrualAccounts,
};
use crate::instructions::market_info::{
    load_main_offer, refresh_market_stats_pda, refresh_modular_market_stats,
};
use crate::instructions::offer::offer_utils::should_accrue_onyc_mint;
use crate::instructions::Offer;
use crate::model::{seeds::MANAGED_TOKEN, ManagedTokenConfig, Pricer};
use crate::state::State;
use crate::utils::token_utils::mint_tokens;
use anchor_lang::{prelude::*, solana_program::program_option::COption};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Event emitted when ONyc tokens are successfully minted to the boss account
///
/// Provides transparency for tracking token minting operations performed by the boss.
#[event]
pub struct OnycTokensMintedEvent {
    /// The ONyc mint from which tokens were minted
    pub onyc_mint: Pubkey,
    /// The boss account that received the newly minted tokens
    pub boss: Pubkey,
    /// The amount of tokens minted in base units
    pub amount: u64,
}

/// Account structure for minting ONyc tokens to the boss
///
/// This struct defines the accounts required for the boss to mint new ONyc tokens
/// to their own account. Requires program mint authority and boss authorization.
#[derive(Accounts)]
pub struct MintTo<'info> {
    /// The program state account containing boss and ONyc mint validation
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss,
        has_one = onyc_mint,
        constraint = !state.is_killed @ crate::OnreError::KillSwitchActivated,
        constraint = state.main_offer != Pubkey::default() @ crate::OnreError::InvalidMainOffer
    )]
    pub state: Box<Account<'info, State>>,

    /// The boss authorized to perform minting operations
    ///
    /// Must be the boss stored in the program state and pay for any
    /// account creation if the token account doesn't exist.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// The ONyc token mint account for minting new tokens
    ///
    /// Must match the ONyc mint stored in program state and be mutable
    /// to allow supply updates during minting.
    #[account(mut)]
    pub onyc_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The boss's ONyc token account to receive minted tokens
    ///
    /// If the account doesn't exist, it will be created automatically
    /// as an Associated Token Account with the boss as the authority.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = onyc_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_program
    )]
    pub boss_onyc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Program-derived account that serves as the mint authority
    ///
    /// This PDA must be the current mint authority for the ONyc token.
    /// Validated to ensure the program has permission to mint tokens.
    /// CHECK: PDA derivation is validated by seeds constraint
    /// CHECK: PDA derivation is validated by seeds constraint and mint authority constraint.
    /// Before the ONyc BUFFER switch it must be the direct mint authority; after it,
    /// the configured program minter (direct or mint multisig) is checked in the handler.
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        constraint = !state.onyc_legacy_buffer_active()
            || onyc_mint.mint_authority == COption::Some(mint_authority.key())
            @ crate::OnreError::NoMintAuthority,
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,

    /// SPL Token program for minting operations
    pub token_program: Interface<'info, TokenInterface>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program required for account creation and rent payment
    pub system_program: Program<'info, System>,

    /// CHECK: Parsed and validated against `state.main_offer` in instruction logic.
    pub main_offer: UncheckedAccount<'info>,

    pub buffer_accounts: BufferAccrualAccounts<'info>,

    /// CHECK: Validated and optionally initialized in instruction logic.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: PDA validation and data loading are handled by market stats refresh.
    pub circulating_supply_excluded_balance: UncheckedAccount<'info>,

    // After `switch_onyc_buffer_to_per_mint`, ONyc mints like a modular path: these
    // accounts are required then and ignored before (pass `None`, i.e. the program
    // id, until the switch).
    /// ONyc's `ManagedTokenConfig`: per-mint caps and the configured mint multisig.
    #[account(
        seeds = [MANAGED_TOKEN, onyc_mint.key().as_ref()],
        bump = managed_token.bump,
    )]
    pub managed_token: Option<Box<Account<'info, ManagedTokenConfig>>>,

    /// CHECK: Required exactly when `ManagedTokenConfig::mint_multisig` is set and must
    /// equal it. See `utils::program_minter`.
    pub mint_multisig: Option<UncheckedAccount<'info>>,

    /// CHECK: Handler validates the canonical `["buffer", onyc_mint]` identity,
    /// owner, discriminator and embedded mint.
    #[account(mut)]
    pub per_mint_buffer_state: Option<UncheckedAccount<'info>>,

    /// Canonical USD instance-zero Pricer driving the per-mint BUFFER.
    pub buffer_pricer: Option<Box<Account<'info, Pricer>>>,

    /// CHECK: Handler validates the per-mint BUFFER's reserve ATA.
    #[account(mut)]
    pub per_mint_reserve_token: Option<UncheckedAccount<'info>>,

    /// CHECK: Handler validates the per-mint BUFFER's management-fee ATA.
    #[account(mut)]
    pub per_mint_management_token: Option<UncheckedAccount<'info>>,

    /// CHECK: Handler validates the per-mint BUFFER's performance-fee ATA.
    #[account(mut)]
    pub per_mint_performance_token: Option<UncheckedAccount<'info>>,

    /// CHECK: ONyc's selected `ManagedTokenConfig::market_stats`, validated by the
    /// refresh; `None` when nothing or the legacy singleton is selected.
    #[account(mut)]
    pub managed_market_stats: Option<UncheckedAccount<'info>>,

    /// CHECK: ONyc's `["supply_exclusions", onyc_mint]`, validated by the refresh.
    pub supply_exclusions: Option<UncheckedAccount<'info>>,
}

impl<'info> MintTo<'info> {
    /// The per-mint ONyc BUFFER accounts; all of them are required after the switch.
    fn per_mint_buffer(&self) -> Result<BufferContext<'_, 'info>> {
        let (
            Some(managed_token),
            Some(buffer_state),
            Some(buffer_pricer),
            Some(reserve_token),
            Some(management_token),
            Some(performance_token),
        ) = (
            self.managed_token.as_deref(),
            self.per_mint_buffer_state.as_ref(),
            self.buffer_pricer.as_deref(),
            self.per_mint_reserve_token.as_ref(),
            self.per_mint_management_token.as_ref(),
            self.per_mint_performance_token.as_ref(),
        )
        else {
            return err!(crate::OnreError::PerMintBufferAccountsRequired);
        };
        Ok(BufferContext {
            state: &self.state,
            managed_mint: &self.onyc_mint,
            managed_program: &self.token_program,
            buffer_state,
            buffer_pricer: Some(buffer_pricer),
            reserve_token,
            management_token,
            performance_token,
            mint_authority: &self.mint_authority,
            configured_mint_multisig: managed_token.mint_multisig(),
            mint_multisig: self.mint_multisig.as_ref(),
        })
    }
}

/// Mints new ONyc tokens directly to the boss's account
///
/// This instruction allows the boss to create new ONyc tokens and add them to their
/// own token account. The operation requires the program to have mint authority for
/// the ONyc token, which must be transferred via `transfer_mint_authority_to_program`.
///
/// The boss's token account is created automatically if it doesn't exist. The minting
/// operation increases the total supply of ONyc tokens and emits an event for tracking.
/// `state.main_offer` must be configured, and the matching offer account must be supplied.
///
/// The mint first accrues the active ONyc BUFFER. Before
/// `switch_onyc_buffer_to_per_mint` that is the legacy singleton, under the `State`
/// caps, minting directly with the program PDA. After it, `mint_to` mints like the
/// modular paths: it accrues the per-mint `["buffer", onyc_mint]` BUFFER, mints
/// under ONyc's `ManagedTokenConfig` caps with the configured program minter, and
/// refreshes market stats from the canonical Pricer, so it keeps working after the
/// legacy offers' vectors are deleted.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `amount` - The amount of ONyc tokens to mint in base units
///
/// # Returns
/// * `Ok(())` - If minting completes successfully
/// * `Err(crate::OnreError::NoMintAuthority)` - If program lacks mint authority
/// * `Err(_)` - If token minting operation fails
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Program must have mint authority for the ONyc token
/// - Boss account must match the one stored in program state
///
/// # Events
/// * `OnycTokensMintedEvent` - Emitted on successful minting with details
pub fn mint_to(mut ctx: Context<MintTo>, amount: u64) -> Result<()> {
    if ctx.accounts.state.onyc_legacy_buffer_active() {
        let offer = load_main_offer(
            ctx.program_id,
            &ctx.accounts.main_offer.to_account_info(),
            &ctx.accounts.state,
        )?;
        mint_with_legacy_buffer(&mut ctx, &offer, amount)?;
        ctx.accounts.onyc_mint.reload()?;
        let a = &ctx.accounts;
        refresh_market_stats_pda(
            &offer,
            &a.onyc_mint,
            &a.circulating_supply_excluded_balance.to_account_info(),
            &a.market_stats.to_account_info(),
            &a.boss.to_account_info(),
            &a.system_program.to_account_info(),
            ctx.program_id,
        )?;
    } else {
        mint_with_per_mint_buffer(&mut ctx, amount)?;
        refresh_per_mint_market_stats(&ctx)?;
    }

    msg!("Minted {} ONyc tokens to boss account", amount);

    // Emit event for transparency and off-chain tracking
    emit!(OnycTokensMintedEvent {
        onyc_mint: ctx.accounts.onyc_mint.key(),
        boss: ctx.accounts.boss.key(),
        amount,
    });

    Ok(())
}

/// Before the switch: accrue the legacy singleton (when initialized and the program
/// controls the mint), mint, then store the post-mint supply on the singleton.
fn mint_with_legacy_buffer<'a, 'info>(
    ctx: &mut Context<'a, MintTo<'info>>,
    offer: &Offer,
    amount: u64,
) -> Result<()> {
    let buffer_is_initialized = ctx
        .accounts
        .buffer_accounts
        .check_is_initialized(ctx.program_id)?;
    let should_accrue = should_accrue_onyc_mint(
        &ctx.accounts.state,
        &ctx.accounts.onyc_mint,
        buffer_is_initialized,
        &ctx.accounts.mint_authority.to_account_info(),
    );
    let accrual = if should_accrue {
        Some(accrue_buffer_from_accounts(
            ctx.program_id,
            &ctx.accounts.state,
            &ctx.accounts.buffer_accounts,
            offer,
            &ctx.accounts.onyc_mint,
            ctx.accounts.mint_authority.to_account_info(),
            ctx.bumps.mint_authority,
            &ctx.accounts.token_program,
        )?)
    } else {
        None
    };

    if accrual.is_some() {
        ctx.accounts.onyc_mint.reload()?;
    }

    let mint_authority_seeds = &[seeds::MINT_AUTHORITY, &[ctx.bumps.mint_authority]];
    let mint_authority_signer_seeds = &[mint_authority_seeds.as_slice()];
    mint_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.onyc_mint,
        &ctx.accounts.boss_onyc_account.to_account_info(),
        &ctx.accounts.mint_authority.to_account_info(),
        mint_authority_signer_seeds,
        amount,
        ctx.accounts.state.max_supply,
        ctx.accounts.state.max_mint_amount,
    )?;

    if let Some(accrual) = accrual {
        let post_mint_supply = accrual
            .post_accrual_supply
            .checked_add(amount)
            .ok_or(crate::OnreError::MathOverflow)?;
        store_buffer_post_supply(
            &ctx.accounts.buffer_accounts,
            post_mint_supply,
            accrual.timestamp,
        )?;
    }
    Ok(())
}

/// After the switch: accrue the per-mint BUFFER up to now, mint with the configured
/// program minter under the per-mint caps, then move the BUFFER's supply baseline to
/// the post-mint supply so the minted amount does not accrue for the past interval.
fn mint_with_per_mint_buffer<'a, 'info>(
    ctx: &mut Context<'a, MintTo<'info>>,
    amount: u64,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let bump = ctx.bumps.mint_authority;
    let (max_supply, max_mint) = ctx
        .accounts
        .managed_token
        .as_ref()
        .ok_or(crate::OnreError::PerMintBufferAccountsRequired)?
        .mint_limits();
    let accrued = modular_accrual::accrue(
        ctx.program_id,
        &ctx.accounts.per_mint_buffer()?,
        bump,
        now,
        max_supply,
        max_mint,
    )?;
    ctx.accounts.onyc_mint.reload()?;

    {
        let a = &ctx.accounts;
        a.per_mint_buffer()?.minter(bump)?.mint(
            &a.token_program,
            &a.onyc_mint,
            &a.boss_onyc_account.to_account_info(),
            amount,
            max_supply,
            max_mint,
        )?;
    }
    ctx.accounts.onyc_mint.reload()?;

    if accrued {
        modular_accrual::store_post_supply(ctx.program_id, &ctx.accounts.per_mint_buffer()?, now)?;
    }
    Ok(())
}

/// After the switch: report like the modular paths, from the canonical Pricer and
/// ONyc's per-mint supply exclusions, so `mint_to` no longer needs the main offer's
/// vectors (deleted once the legacy offers are retired).
fn refresh_per_mint_market_stats<'a, 'info>(ctx: &Context<'a, MintTo<'info>>) -> Result<()> {
    let a = &ctx.accounts;
    let (Some(managed_token), Some(supply_exclusions)) =
        (a.managed_token.as_deref(), a.supply_exclusions.as_ref())
    else {
        return err!(crate::OnreError::PerMintBufferAccountsRequired);
    };
    let legacy = a.market_stats.to_account_info();
    // An unselected (or singleton) selection never reads this account.
    let selected = a
        .managed_market_stats
        .as_ref()
        .map_or_else(|| legacy.clone(), |account| account.to_account_info());
    let payer = a.boss.to_account_info();
    let system_program = a.system_program.to_account_info();
    refresh_modular_market_stats(
        ctx.program_id,
        &a.state,
        managed_token,
        &a.onyc_mint,
        &supply_exclusions.to_account_info(),
        &selected,
        Some(&legacy),
        Some((&payer, &system_program)),
        || modular_accrual::canonical_market_valuation(ctx.program_id, &a.per_mint_buffer()?),
    )
}
