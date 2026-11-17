use crate::{
    constants::seeds,
    model::{
        seeds::{MANAGED_TOKEN, OFFER_CONFIG},
        FeeConfig, FulfillmentRequest, ManagedTokenConfig, OfferAccess, OfferConfig,
        OfferDirection, OfferExecution, Pricer, PricingDenomination, PropAmmState, Quoter,
        VaultConfig, VaultKind,
    },
    state::State,
    utils::program_minter::ProgramMinter,
    OnreError,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use solana_instructions_sysvar::ID as INSTRUCTIONS_SYSVAR_ID;

/// Shared accounts for modular offer execution in either direction.
///
/// OfferConfig selects direction and permissions; Quoter selects pricing. Optional
/// extension accounts are required and validated when the configuration uses them.
/// Execution support remains limited to the implemented settlement paths.
#[derive(Accounts)]
pub struct TakeOffer<'info> {
    /// Offer components. A separate `try_accounts` frame keeps validation within the
    /// SBF stack limit; the mint accounts below must equal the stored mint pair.
    pub config: TakeOfferConfigAccounts<'info>,

    #[account(
        mut,
        address = config.offer_config.token_in_mint @ OnreError::InvalidTokenInMint,
        mint::token_program = token_in_program
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        address = config.offer_config.token_out_mint @ OnreError::InvalidTokenOutMint,
        mint::token_program = token_out_program
    )]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = user,
        associated_token::token_program = token_in_program
    )]
    pub user_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_out_mint,
        associated_token::authority = user,
        associated_token::token_program = token_out_program
    )]
    pub user_token_out: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_in_mint,
        associated_token::authority = config.proceeds_vault,
        associated_token::token_program = token_in_program
    )]
    pub proceeds_token_in: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_in_mint,
        associated_token::authority = config.fee_vault,
        associated_token::token_program = token_in_program
    )]
    pub fee_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Canonical program mint authority; mint authority is checked in the handler.
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,

    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = !state.is_killed @ OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// Canonical per-mint BUFFER state, Pricer, reporting and BUFFER vault ATAs.
    /// A separate `try_accounts` frame keeps validation within the SBF stack limit.
    pub buffer: TakeOfferBufferAccounts<'info>,

    /// CHECK: Ed25519 approval verification reads the canonical instructions sysvar.
    #[account(address = INSTRUCTIONS_SYSVAR_ID @ OnreError::InvalidInstructionsSysvar)]
    pub instructions_sysvar: UncheckedAccount<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_in_program: Interface<'info, TokenInterface>,
    pub token_out_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,

    /// Pressure state and configured source/destination custody.
    /// A separate `try_accounts` frame keeps validation within the SBF stack limit.
    pub custody: TakeOfferCustodyAccounts<'info>,

    /// CHECK: Canonical intermediary PDA, required only by permissionless offers.
    /// Its address and token authorities are checked before execution.
    pub permissionless_authority: Option<UncheckedAccount<'info>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_in_mint,
        associated_token::authority = permissionless_authority,
        associated_token::token_program = token_in_program
    )]
    pub permissionless_token_in: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = token_out_mint,
        associated_token::authority = permissionless_authority,
        associated_token::token_program = token_out_program
    )]
    pub permissionless_token_out: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
}

/// Contiguous `TakeOffer` accounts, flattened in place on the wire.
#[derive(Accounts)]
pub struct TakeOfferBufferAccounts<'info> {
    /// CHECK: Always the canonical `["buffer", managed_mint]` address, ONyc included;
    /// used whenever it is the active BUFFER. A substitute cannot bypass initialized
    /// accrual. ONyc before the one-way switch accrues the legacy accounts below.
    #[account(mut)]
    pub buffer_state: UncheckedAccount<'info>,

    /// Canonical USD instance-zero Pricer, required for initialized per-mint BUFFER,
    /// and for token-wide liquidity targets or market stats when the offer selects
    /// another Pricer. The offer's selected Pricer does not control token-wide values.
    pub buffer_pricer: Option<Box<Account<'info, Pricer>>>,

    /// CHECK: Must equal `ManagedTokenConfig::market_stats` when one is selected;
    /// ignored otherwise.
    #[account(mut)]
    pub market_stats: UncheckedAccount<'info>,

    /// CHECK: Legacy ONyc `["market_stats"]` singleton read by integrators. Required
    /// when the managed mint is ONyc, which always refreshes it from the canonical
    /// Pricer (address validated in the handler, created on first use with `user`
    /// paying); ignored for other mints.
    #[account(mut)]
    pub legacy_market_stats: Option<UncheckedAccount<'info>>,

    /// CHECK: ONyc refresh and targets validate the canonical excluded-balance cache;
    /// other mints ignore it.
    pub circulating_supply_excluded_balance: UncheckedAccount<'info>,

    /// CHECK: When BUFFER is active, its expected reserve ATA is validated in the handler.
    #[account(mut)]
    pub buffer_reserve_token: UncheckedAccount<'info>,

    /// CHECK: When BUFFER is active, its expected management-fee ATA is validated.
    #[account(mut)]
    pub buffer_management_token: UncheckedAccount<'info>,

    /// CHECK: When BUFFER is active, its expected performance-fee ATA is validated.
    #[account(mut)]
    pub buffer_performance_token: UncheckedAccount<'info>,

    /// CHECK: Required exactly when `ManagedTokenConfig::mint_multisig` is set and must
    /// equal it; omitted for direct PDA minting. See `utils::program_minter`.
    pub mint_multisig: Option<UncheckedAccount<'info>>,

    /// CHECK: Legacy ONyc `["buffer_state"]` singleton. Required for ONyc before the
    /// one-way BUFFER switch, which it then accrues; ignored afterwards and for other
    /// mints. Clients can supply it alongside the per-mint BUFFER accounts at all
    /// times, so the switch needs no account changes.
    #[account(mut)]
    pub legacy_buffer_state: Option<UncheckedAccount<'info>>,

    /// CHECK: Legacy reserve vault's ONyc ATA; same rule as `legacy_buffer_state`.
    #[account(mut)]
    pub legacy_buffer_reserve_token: Option<UncheckedAccount<'info>>,

    /// CHECK: Legacy management-fee vault's ONyc ATA; same rule as `legacy_buffer_state`.
    #[account(mut)]
    pub legacy_buffer_management_token: Option<UncheckedAccount<'info>>,

    /// CHECK: Legacy performance-fee vault's ONyc ATA; same rule as `legacy_buffer_state`.
    #[account(mut)]
    pub legacy_buffer_performance_token: Option<UncheckedAccount<'info>>,
}

/// Contiguous `TakeOffer` accounts, flattened in place on the wire.
#[derive(Accounts)]
pub struct TakeOfferCustodyAccounts<'info> {
    /// Required exactly when OfferConfig.pressure_state is configured, including
    /// linked NAV offers. Validated against the stored reference and mint pair.
    #[account(mut)]
    pub prop_amm_state: Option<Box<Account<'info, PropAmmState>>>,

    /// CHECK: Exact configured output custody; validates identity and signing seeds.
    pub source_vault: Option<UncheckedAccount<'info>>,

    /// CHECK: Canonical output-mint ATA of source_vault; empty means no inventory.
    #[account(mut)]
    pub source_token_out: Option<UncheckedAccount<'info>>,

    /// CHECK: Exact configured input custody; validates identity and refill policy.
    pub destination_vault: Option<UncheckedAccount<'info>>,

    /// CHECK: Canonical input-mint ATA of destination_vault, initialized on deposit.
    #[account(mut)]
    pub destination_token_in: Option<UncheckedAccount<'info>>,

    /// CHECK: Exact configured Haircut vault; required exactly when the offer has one.
    pub haircut_vault: Option<UncheckedAccount<'info>>,

    /// CHECK: Canonical input-mint ATA of haircut_vault, initialized on first deposit.
    #[account(mut)]
    pub haircut_token_in: Option<UncheckedAccount<'info>>,
}

/// Contiguous `TakeOffer` accounts, flattened in place on the wire.
#[derive(Accounts)]
pub struct TakeOfferConfigAccounts<'info> {
    #[account(
        seeds = [
            OFFER_CONFIG,
            offer_config.token_in_mint.as_ref(),
            offer_config.token_out_mint.as_ref(),
            &offer_config.instance_id.to_le_bytes()
        ],
        bump = offer_config.bump,
        constraint = offer_config.enabled @ OnreError::ConfigurationDisabled,
        has_one = pricer @ OnreError::IncompatiblePricer,
        has_one = quoter @ OnreError::IncompatibleQuoter,
        has_one = fee_config @ OnreError::InvalidFee,
        has_one = proceeds_vault @ OnreError::InvalidConfigurableVault
    )]
    pub offer_config: Box<Account<'info, OfferConfig>>,

    #[account(
        seeds = [MANAGED_TOKEN, managed_token.managed_mint.as_ref()],
        bump = managed_token.bump,
        constraint = managed_token.enabled @ OnreError::ConfigurationDisabled
    )]
    pub managed_token: Box<Account<'info, ManagedTokenConfig>>,

    #[account(
        constraint = pricer.enabled @ OnreError::ConfigurationDisabled,
        constraint = pricer.managed_mint == managed_token.managed_mint @ OnreError::IncompatiblePricer,
        constraint = pricer.denomination == PricingDenomination::Usd @ OnreError::IncompatiblePricer
    )]
    pub pricer: Box<Account<'info, Pricer>>,

    #[account(
        constraint = quoter.enabled @ OnreError::ConfigurationDisabled
    )]
    pub quoter: Box<Account<'info, Quoter>>,

    /// Re-checked against the offer's input decimals, which execution also checks
    /// against the input mint.
    #[account(
        constraint = fee_config.enabled @ OnreError::ConfigurationDisabled,
        constraint = fee_config.token_decimals == offer_config.token_in_decimals
            @ OnreError::FeeConfigDecimalsMismatch,
        has_one = fee_vault @ OnreError::InvalidConfigurableVault
    )]
    pub fee_config: Box<Account<'info, FeeConfig>>,

    #[account(
        constraint = proceeds_vault.kind == VaultKind::Proceeds @ OnreError::InvalidConfigurableVaultKind
    )]
    pub proceeds_vault: Box<Account<'info, VaultConfig>>,

    #[account(
        constraint = fee_vault.kind == VaultKind::Fee @ OnreError::InvalidConfigurableVaultKind
    )]
    pub fee_vault: Box<Account<'info, VaultConfig>>,
}

/// Borrowed view of everything one offer execution needs. `take_offer` and
/// `fulfill_fulfillment_request` build it from their own accounts, so both share
/// validation, quoting, BUFFER accrual, settlement and reporting.
pub(crate) struct Execution<'a, 'info> {
    pub config: &'a TakeOfferConfigAccounts<'info>,
    pub token_in_mint: &'a InterfaceAccount<'info, Mint>,
    pub token_out_mint: &'a InterfaceAccount<'info, Mint>,
    pub proceeds_token_in: Option<&'a InterfaceAccount<'info, TokenAccount>>,
    pub fee_token_in: &'a InterfaceAccount<'info, TokenAccount>,
    pub mint_authority: &'a UncheckedAccount<'info>,
    pub state: &'a Account<'info, State>,
    pub buffer: &'a TakeOfferBufferAccounts<'info>,
    pub custody: &'a TakeOfferCustodyAccounts<'info>,
    pub token_in_program: &'a Interface<'info, TokenInterface>,
    pub token_out_program: &'a Interface<'info, TokenInterface>,
    pub associated_token_program: &'a Program<'info, AssociatedToken>,
    pub system_program: &'a Program<'info, System>,
    /// Pays for accounts created during execution: the trading user on
    /// `take_offer`, the worker on a fill.
    pub payer: &'a AccountInfo<'info>,
    /// Trader credited with the output and reported in events.
    pub user: Pubkey,
    pub route: Route<'a, 'info>,
}

/// Where the gross input comes from and the output goes, by execution mode.
#[derive(Clone, Copy)]
pub(crate) enum Route<'a, 'info> {
    /// Permissioned instant: the signing user's own ATAs.
    Direct {
        input: &'a InterfaceAccount<'info, TokenAccount>,
        output: &'a InterfaceAccount<'info, TokenAccount>,
        user: &'a AccountInfo<'info>,
    },
    /// Permissionless instant: through the program's intermediary ATAs.
    Intermediary {
        user_input: &'a InterfaceAccount<'info, TokenAccount>,
        user_output: &'a InterfaceAccount<'info, TokenAccount>,
        user: &'a AccountInfo<'info>,
        authority: &'a AccountInfo<'info>,
        input: &'a InterfaceAccount<'info, TokenAccount>,
        output: &'a InterfaceAccount<'info, TokenAccount>,
    },
    /// Worker: the fulfillment request signs for its escrow; output goes to the
    /// request user's ATA.
    Escrow {
        request: &'a Account<'info, FulfillmentRequest>,
        input: &'a InterfaceAccount<'info, TokenAccount>,
        output: &'a InterfaceAccount<'info, TokenAccount>,
    },
}

impl Route<'_, '_> {
    /// The only offer modes each route may execute.
    pub fn supports(&self, offer: &OfferConfig) -> bool {
        match self {
            Route::Direct { .. } => {
                (offer.execution, offer.access)
                    == (OfferExecution::Instant, OfferAccess::Permissioned)
            }
            Route::Intermediary { .. } => {
                (offer.execution, offer.access)
                    == (OfferExecution::Instant, OfferAccess::Permissionless)
            }
            Route::Escrow { .. } => offer.execution == OfferExecution::Worker,
        }
    }
}

/// Accounts structs that can run an offer execution.
pub(crate) trait ExecutionAccounts<'info> {
    fn execution(&self) -> Result<Execution<'_, 'info>>;
    fn prop_amm_state_mut(&mut self) -> Option<&mut Account<'info, PropAmmState>>;
    /// Re-reads the managed mint after CPIs changed its supply.
    fn reload_managed_mint(&mut self) -> Result<()>;
}

impl<'a, 'info> Execution<'a, 'info> {
    /// Program minting path for the managed mint in its configured mode.
    pub(super) fn minter(&self, authority_bump: u8) -> Result<ProgramMinter<'a, 'info>> {
        ProgramMinter::resolve(
            self.config.managed_token.mint_multisig(),
            self.managed_mint(),
            self.mint_authority.as_ref(),
            authority_bump,
            self.buffer.mint_multisig.as_ref().map(AsRef::as_ref),
        )
    }

    /// Resolve the managed side from persisted configuration, never caller input.
    pub(super) fn managed_mint(&self) -> &'a InterfaceAccount<'info, Mint> {
        match self.config.offer_config.direction {
            OfferDirection::AssetToManaged => self.token_out_mint,
            OfferDirection::ManagedToAsset => self.token_in_mint,
        }
    }

    pub(super) fn managed_program(&self) -> &'a Interface<'info, TokenInterface> {
        match self.config.offer_config.direction {
            OfferDirection::AssetToManaged => self.token_out_program,
            OfferDirection::ManagedToAsset => self.token_in_program,
        }
    }

    pub(super) fn asset_mint(&self) -> &'a InterfaceAccount<'info, Mint> {
        match self.config.offer_config.direction {
            OfferDirection::AssetToManaged => self.token_in_mint,
            OfferDirection::ManagedToAsset => self.token_out_mint,
        }
    }
}

/// Reloads whichever of the two mints the offer manages.
pub(crate) fn reload_managed_mint<'info>(
    offer: &OfferConfig,
    token_in_mint: &mut InterfaceAccount<'info, Mint>,
    token_out_mint: &mut InterfaceAccount<'info, Mint>,
) -> Result<()> {
    match offer.direction {
        OfferDirection::AssetToManaged => token_out_mint.reload(),
        OfferDirection::ManagedToAsset => token_in_mint.reload(),
    }
}

impl<'info> ExecutionAccounts<'info> for TakeOffer<'info> {
    fn execution(&self) -> Result<Execution<'_, 'info>> {
        Ok(Execution {
            config: &self.config,
            token_in_mint: &self.token_in_mint,
            token_out_mint: &self.token_out_mint,
            proceeds_token_in: self.proceeds_token_in.as_deref(),
            fee_token_in: &self.fee_token_in,
            mint_authority: &self.mint_authority,
            state: &self.state,
            buffer: &self.buffer,
            custody: &self.custody,
            token_in_program: &self.token_in_program,
            token_out_program: &self.token_out_program,
            associated_token_program: &self.associated_token_program,
            system_program: &self.system_program,
            payer: self.user.as_ref(),
            user: self.user.key(),
            route: self.route()?,
        })
    }

    fn prop_amm_state_mut(&mut self) -> Option<&mut Account<'info, PropAmmState>> {
        self.custody.prop_amm_state.as_deref_mut()
    }

    fn reload_managed_mint(&mut self) -> Result<()> {
        reload_managed_mint(
            &self.config.offer_config,
            &mut self.token_in_mint,
            &mut self.token_out_mint,
        )
    }
}

impl<'info> TakeOffer<'info> {
    /// The stored access mode selects the route; caller-supplied accounts cannot
    /// switch it. Permissionless offers require the intermediary and its ATAs
    /// (validated by `validate_intermediary`); every other offer must omit them.
    fn route(&self) -> Result<Route<'_, 'info>> {
        if self.config.offer_config.access != OfferAccess::Permissionless {
            require!(
                self.permissionless_authority.is_none()
                    && self.permissionless_token_in.is_none()
                    && self.permissionless_token_out.is_none(),
                OnreError::PermissionlessNotAllowed
            );
            return Ok(Route::Direct {
                input: &self.user_token_in,
                output: &self.user_token_out,
                user: self.user.as_ref(),
            });
        }
        Ok(Route::Intermediary {
            user_input: &self.user_token_in,
            user_output: &self.user_token_out,
            user: self.user.as_ref(),
            authority: self
                .permissionless_authority
                .as_ref()
                .ok_or(OnreError::WrongAuthority)?
                .as_ref(),
            input: self
                .permissionless_token_in
                .as_deref()
                .ok_or(OnreError::InvalidVaultTokenInAccount)?,
            output: self
                .permissionless_token_out
                .as_deref()
                .ok_or(OnreError::InvalidPermissionlessTokenOutAccount)?,
        })
    }
}
