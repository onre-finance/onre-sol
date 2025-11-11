use crate::constants::seeds;
use crate::instructions::Offer;
use crate::state::State;
use crate::OfferCoreError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

/// Event emitted when an offer is successfully closed and account is reclaimed
///
/// Provides transparency for tracking offer lifecycle and account cleanup operations.
#[event]
pub struct OfferClosedEvent {
    /// The PDA address of the offer that was closed
    pub offer_pda: Pubkey,
    /// The boss account that initiated the closure and received the rent
    pub boss: Pubkey,
}

/// Account structure for closing an offer and reclaiming rent
///
/// This struct defines the accounts required to permanently close an offer
/// and transfer its rent balance back to the boss. Only the boss can close offers.
#[derive(Accounts)]
pub struct CloseOffer<'info> {
    /// The offer account to be closed and its rent reclaimed
    ///
    /// This account is validated as a PDA derived from token mint addresses.
    /// The account will be closed and its rent transferred to the boss.
    #[account(
        mut,
        seeds = [
            seeds::OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump = offer.load()?.bump,
        close = boss
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// The input token mint account for offer validation
    #[account(
        constraint =
            token_in_mint.key() == offer.load()?.token_in_mint
            @ OfferCoreError::InvalidTokenInMint
    )]
    pub token_in_mint: InterfaceAccount<'info, Mint>,

    /// The output token mint account for offer validation
    #[account(
        constraint =
            token_out_mint.key() == offer.load()?.token_out_mint
            @ OfferCoreError::InvalidTokenOutMint
    )]
    pub token_out_mint: InterfaceAccount<'info, Mint>,

    /// The boss account authorized to close offers and receive rent
    pub boss: Signer<'info>,

    /// Program state account containing boss authorization
    #[account(seeds = [seeds::STATE], bump = state.bump, has_one = boss)]
    pub state: Account<'info, State>,

    /// System program required for account closure and rent transfer
    pub system_program: Program<'info, System>,
}

/// Permanently closes an offer and reclaims its rent balance
///
/// This instruction removes an offer from the protocol and transfers its rent
/// balance back to the boss. The offer account is permanently deleted and cannot
/// be recovered. All pricing vectors and configuration are lost.
///
/// This operation is useful for cleaning up inactive offers and reclaiming
/// storage rent when offers are no longer needed.
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If the offer is successfully closed and rent reclaimed
///
/// # Access Control
/// - Only the boss can call this instruction
/// - Boss account must match the one stored in program state
/// - Offer must belong to the specified token pair
///
/// # Effects
/// - Offer account is permanently deleted
/// - Rent balance is transferred to the boss
/// - All pricing vectors and offer data are lost
///
/// # Events
/// * `CloseOfferEvent` - Emitted with offer PDA and boss details
pub fn close_offer(ctx: Context<CloseOffer>) -> Result<()> {
    emit!(OfferClosedEvent {
        offer_pda: ctx.accounts.offer.key(),
        boss: ctx.accounts.boss.key(),
    });

    Ok(())
}
