use crate::constants::{seeds, MAX_ALLOWED_FEE_BPS};
use crate::instructions::Offer;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Event emitted when an offer is successfully created
///
/// Provides transparency for tracking offer creation and configuration parameters.
#[event]
pub struct OfferMadeEvent {
    /// The PDA address of the newly created offer
    pub offer_pda: Pubkey,
    /// The input token mint for the offer
    pub token_in_mint: Pubkey,
    /// The output token mint for the offer
    pub token_out_mint: Pubkey,
    /// Fee in basis points (10000 = 100%) charged when taking the offer
    pub fee_basis_points: u16,
    /// The boss account that created and owns the offer
    pub boss: Pubkey,
    /// Whether the offer requires boss approval for taking
    pub needs_approval: bool,
    /// Whether the offer allows permissionless operations
    pub allow_permissionless: bool,
}

/// Account structure for creating an offer
///
/// This struct defines the accounts required to initialize a token exchange offer
/// where the boss provides token_in in exchange for token_out. Pricing is configured
/// separately using pricing vectors after offer creation.
#[derive(Accounts)]
pub struct MakeOffer<'info> {
    /// Program-derived authority that controls offer vault token accounts
    ///
    /// This PDA manages token transfers and burning operations when the program
    /// has mint authority for efficient burn/mint architecture.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::OFFER_VAULT_AUTHORITY], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    /// The input token mint for the offer
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    /// Token program interface for the input token
    pub token_in_program: Interface<'info, TokenInterface>,

    /// Vault account for storing input tokens during burn/mint operations
    ///
    /// Created automatically if needed. Used for temporary token storage
    /// when the program has mint authority and needs to burn tokens.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_in_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_in_program
    )]
    pub vault_token_in_account: InterfaceAccount<'info, TokenAccount>,

    /// The output token mint for the offer
    pub token_out_mint: InterfaceAccount<'info, Mint>,

    /// The offer account storing exchange configuration and pricing vectors
    ///
    /// This account is derived from token mint addresses ensuring unique
    /// offers per token pair. Contains fee settings, approval requirements,
    /// and pricing vector array for dynamic pricing.
    #[account(
        init,
        payer = boss,
        space = 8 + Offer::INIT_SPACE,
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// Program state account containing boss authorization
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = boss)]
    pub state: Account<'info, State>,

    /// The boss account authorized to create offers and pay for account creation
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program for account creation and rent payment
    pub system_program: Program<'info, System>,
}

/// Creates a token exchange offer
///
/// This instruction initializes a new offer where the boss provides token_in in exchange
/// for token_out. The offer is created with basic configuration parameters, and pricing
/// is configured separately using add_offer_vector instructions for dynamic pricing.
///
/// The offer is uniquely identified by the token pair and supports configurable fees,
/// approval requirements, and permissionless operation settings.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `fee_basis_points` - Fee in basis points (10000 = 100%) charged when taking the offer
/// * `needs_approval` - Whether the offer requires boss approval for taking
/// * `allow_permissionless` - Whether the offer allows permissionless operations
///
/// # Returns
/// * `Ok(())` - If the offer is successfully created
/// * `Err(MakeOfferErrorCode::InvalidFee)` - If fee_basis_points exceeds 10000
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
///
/// # Effects
/// - Creates new offer account with specified configuration
/// - Initializes vault token account if needed for burn/mint operations
/// - Sets up offer parameters for future pricing vector additions
///
/// # Events
/// * `OfferMadeEvent` - Emitted with offer details and configuration
pub fn make_offer(
    ctx: Context<MakeOffer>,
    fee_basis_points: u16,
    needs_approval: bool,
    allow_permissionless: bool,
) -> Result<()> {
    // Validate fee is within valid range (0-10000 basis points = 0-100%)
    require!(
        fee_basis_points <= MAX_ALLOWED_FEE_BPS,
        MakeOfferErrorCode::InvalidFee
    );

    // Create the offer
    let mut offer = ctx.accounts.offer.load_init()?;
    offer.token_in_mint = ctx.accounts.token_in_mint.key();
    offer.token_out_mint = ctx.accounts.token_out_mint.key();
    offer.fee_basis_points = fee_basis_points;
    offer.set_approval(needs_approval);
    offer.set_permissionless(allow_permissionless);
    offer.bump = ctx.bumps.offer;

    msg!("Offer created at: {}", ctx.accounts.offer.key());

    emit!(OfferMadeEvent {
        offer_pda: ctx.accounts.offer.key(),
        token_in_mint: ctx.accounts.token_in_mint.key(),
        token_out_mint: ctx.accounts.token_out_mint.key(),
        fee_basis_points,
        boss: ctx.accounts.boss.key(),
        needs_approval,
        allow_permissionless,
    });

    Ok(())
}

/// Error codes for offer creation operations
#[error_code]
pub enum MakeOfferErrorCode {
    /// The offer account is full and cannot accommodate more offers
    #[msg("Offer account is full, cannot create more offers")]
    AccountFull,

    /// Fee basis points exceeds maximum allowed value of 10000 (100%)
    #[msg("Invalid fee: fee_basis_points must be <= 10000")]
    InvalidFee,

    /// Invalid token program interface provided
    #[msg("Invalid token program")]
    InvalidTokenProgram,
}
