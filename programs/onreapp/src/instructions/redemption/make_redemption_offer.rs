use crate::constants::{seeds, MAX_ALLOWED_FEE_BPS};
use crate::instructions::redemption::RedemptionOffer;
use crate::instructions::Offer;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Event emitted when a redemption offer is successfully created
///
/// Provides transparency for tracking redemption offer creation and configuration.
#[event]
pub struct RedemptionOfferCreatedEvent {
    /// The PDA address of the newly created redemption offer
    pub redemption_offer_pda: Pubkey,
    /// Reference to the original offer
    pub offer: Pubkey,
    /// The input token mint for redemptions (ONyc)
    pub token_in_mint: Pubkey,
    /// The output token mint for redemptions (e.g., USDC)
    pub token_out_mint: Pubkey,
    /// Fee in basis points, capped at 1000 bps (10%), charged when fulfilling redemption requests.
    pub fee_basis_points: u16,
    /// Fee in basis points, capped at 1000 bps (10%), charged when Prop AMM sell fulfills redemptions.
    pub fee_basis_points_prop_amm_sell: u16,
    /// Target token-out vault balance as basis points of TVL.
    pub vault_target_bps: u16,
}

/// Account structure for creating a redemption offer
///
/// This struct defines the accounts required to initialize a redemption offer
/// where users can redeem ONyc tokens for the paired output asset at the current NAV price.
/// The redemption offer is the inverse of the standard Offer.
#[derive(Accounts)]
pub struct MakeRedemptionOffer<'info> {
    /// Program state account containing boss authorization.
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = boss)]
    pub state: Box<Account<'info, State>>,

    /// The original offer that this redemption offer is associated with
    ///
    /// The redemption offer uses the inverse token pair of the original offer.
    /// The offer must be derived from redemption offer token_out_mint (token_in in original offer)
    /// and token_in_mint (token_out in original offer).
    #[account(
        seeds = [
            seeds::OFFER,
            token_out_mint.key().as_ref(),
            token_in_mint.key().as_ref(),
        ],
        bump
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// Program-derived authority that controls redemption offer vault token accounts
    ///
    /// This PDA manages token transfers for redemption operations.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], bump)]
    pub redemption_vault_authority: UncheckedAccount<'info>,

    /// The input token mint for redemptions (token_in_mint)
    ///
    /// This corresponds to the token_out_mint from the original offer.
    #[account(
        constraint = token_in_mint.key() == state.onyc_mint
            @ crate::OnreError::InvalidOnycMint,
        constraint = *token_in_mint.to_account_info().owner == anchor_spl::token::ID
            @ crate::OnreError::InvalidTokenProgram
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for the input token
    #[account(
        constraint = token_in_program.key() == anchor_spl::token::ID
            @ crate::OnreError::InvalidTokenProgram
    )]
    pub token_in_program: Interface<'info, TokenInterface>,

    /// Vault account for storing input tokens during redemption operations
    ///
    /// Created automatically if needed. Used for holding ONyc tokens before burning.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_in_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_in_program
    )]
    pub vault_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The output token mint for redemptions (e.g., USDC)
    ///
    /// This is the token_in_mint from the original offer.
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for the output token
    pub token_out_program: Interface<'info, TokenInterface>,

    /// Vault account for storing output tokens (e.g., USDC) for redemption payouts
    ///
    /// Created automatically if needed. Used for distributing output tokens to redeemers.
    #[account(
        init_if_needed,
        payer = boss,
        associated_token::mint = token_out_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_out_program
    )]
    pub vault_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The redemption offer account storing redemption configuration
    ///
    /// This account is derived from token mint addresses in the same order as Offer
    /// but with the tokens reversed (token_in from Offer becomes token_out here).
    #[account(
        init,
        payer = boss,
        space = 8 + RedemptionOffer::INIT_SPACE,
        seeds = [
            seeds::REDEMPTION_OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump
    )]
    pub redemption_offer: Account<'info, RedemptionOffer>,

    /// Boss creating and funding the redemption offer.
    #[account(mut)]
    pub boss: Signer<'info>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program for account creation and rent payment
    pub system_program: Program<'info, System>,
}

/// Creates a redemption offer for converting ONyc back to the paired output asset.
///
/// This instruction initializes a new redemption offer that allows users to redeem
/// input tokens for output tokens (e.g., USDC) at the current NAV price. The redemption
/// offer is the inverse of the standard Offer - it takes the output token of the standard Offer
/// as input token and provides the input token of the standard Offer as output token.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
/// * `fee_basis_points` - Fee in basis points charged when fulfilling redemption requests
/// * `fee_basis_points_prop_amm_sell` - Fee in basis points charged when Prop AMM sell fulfills redemptions
///
/// # Returns
/// * `Ok(())` - If the redemption offer is successfully created
/// * `Err(crate::OnreError::Unauthorized)` - If caller is not the boss (validated in accounts)
/// * `Err(crate::OnreError::InvalidFee)` - If either fee exceeds 1000 basis points (10%)
///
/// # Access Control
/// - Only the boss can call this instruction
///
/// # Effects
/// - Creates new redemption offer account with reference to the original offer and fee configuration
/// - Initializes vault token accounts if needed for both token_in and token_out
/// - Sets up redemption tracking counters (executed_redemptions, requested_redemptions)
///
/// # Events
/// * `RedemptionOfferCreatedEvent` - Emitted with redemption offer details and configuration
pub fn make_redemption_offer(
    ctx: Context<MakeRedemptionOffer>,
    fee_basis_points: u16,
    fee_basis_points_prop_amm_sell: u16,
) -> Result<()> {
    // Validate fee is within valid range (0-1000 basis points = 0-10%)
    require!(
        fee_basis_points <= MAX_ALLOWED_FEE_BPS,
        crate::OnreError::InvalidFee
    );
    require!(
        fee_basis_points_prop_amm_sell <= MAX_ALLOWED_FEE_BPS,
        crate::OnreError::InvalidFee
    );

    // Initialize the redemption offer
    let redemption_offer = &mut ctx.accounts.redemption_offer;
    redemption_offer.offer = ctx.accounts.offer.key();
    redemption_offer.token_in_mint = ctx.accounts.token_in_mint.key();
    redemption_offer.token_out_mint = ctx.accounts.token_out_mint.key();
    redemption_offer.fee_basis_points = fee_basis_points;
    redemption_offer.fee_basis_points_prop_amm_sell = fee_basis_points_prop_amm_sell;
    redemption_offer.vault_target_bps = 0;
    redemption_offer.executed_redemptions = 0;
    redemption_offer.requested_redemptions = 0;
    redemption_offer.gap = 0;
    redemption_offer.set_disabled(false);
    redemption_offer.bump = ctx.bumps.redemption_offer;

    msg!(
        "Redemption offer created at: {}, fee: {}, prop AMM sell fee: {}",
        ctx.accounts.redemption_offer.key(),
        fee_basis_points,
        fee_basis_points_prop_amm_sell
    );

    emit!(RedemptionOfferCreatedEvent {
        redemption_offer_pda: ctx.accounts.redemption_offer.key(),
        offer: ctx.accounts.offer.key(),
        token_in_mint: ctx.accounts.token_in_mint.key(),
        token_out_mint: ctx.accounts.token_out_mint.key(),
        fee_basis_points,
        fee_basis_points_prop_amm_sell,
        vault_target_bps: 0,
    });

    Ok(())
}
