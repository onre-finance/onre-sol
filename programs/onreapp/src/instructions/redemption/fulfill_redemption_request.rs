use crate::constants::seeds;
use crate::instructions::offer::offer_utils::process_offer_core;
use crate::instructions::redemption::{RedemptionOffer, RedemptionRequest};
use crate::instructions::Offer;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{burn, mint_to, transfer_checked, Burn, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked},
};

/// Event emitted when a redemption request is successfully fulfilled
///
/// Provides transparency for tracking redemption fulfillment and token exchange details.
#[event]
pub struct RedemptionRequestFulfilledEvent {
    /// The PDA address of the fulfilled redemption request
    pub redemption_request_pda: Pubkey,
    /// Reference to the redemption offer
    pub redemption_offer: Pubkey,
    /// User who created the redemption request
    pub redeemer: Pubkey,
    /// Amount of token_in tokens burned/transferred
    pub token_in_amount: u64,
    /// Amount of token_out tokens received by the user
    pub token_out_amount: u64,
    /// Current price used for the redemption
    pub current_price: u64,
}

/// Account structure for fulfilling a redemption request
///
/// This struct defines the accounts required to fulfill a redemption request,
/// handling token burning/transfer for token_in (typically ONyc) and minting/transfer
/// for token_out (typically stablecoins like USDC).
#[derive(Accounts)]
pub struct FulfillRedemptionRequest<'info> {
    /// Program state account containing redemption_admin and boss authorization
    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        has_one = boss @ FulfillRedemptionRequestErrorCode::InvalidBoss,
        constraint = state.is_killed == false @ FulfillRedemptionRequestErrorCode::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// The boss account that may receive tokens when program lacks mint authority
    /// CHECK: Account validation is enforced through state account constraint
    pub boss: UncheckedAccount<'info>,

    /// The underlying offer that defines pricing
    #[account(
        constraint = offer.load()?.token_in_mint == redemption_offer.load()?.token_out_mint
            @ FulfillRedemptionRequestErrorCode::OfferMintMismatch,
        constraint = offer.load()?.token_out_mint == redemption_offer.load()?.token_in_mint
            @ FulfillRedemptionRequestErrorCode::OfferMintMismatch
    )]
    pub offer: AccountLoader<'info, Offer>,

    /// The redemption offer account
    #[account(
        mut,
        constraint = redemption_offer.load()?.offer == offer.key()
            @ FulfillRedemptionRequestErrorCode::OfferMismatch
    )]
    pub redemption_offer: AccountLoader<'info, RedemptionOffer>,

    /// The redemption request account to fulfill
    #[account(
        mut,
        constraint = redemption_request.load()?.status == 0
            @ FulfillRedemptionRequestErrorCode::RequestAlreadyProcessed,
        constraint = redemption_request.load()?.offer == redemption_offer.key()
            @ FulfillRedemptionRequestErrorCode::OfferMismatch
    )]
    pub redemption_request: AccountLoader<'info, RedemptionRequest>,

    /// Program-derived redemption vault authority that controls token operations
    ///
    /// This PDA manages token transfers and burning operations.
    /// CHECK: PDA derivation is validated by seeds constraint
    #[account(
        seeds = [seeds::REDEMPTION_OFFER_VAULT_AUTHORITY],
        bump
    )]
    pub redemption_vault_authority: AccountInfo<'info>,

    /// Redemption vault account for token_in (to receive tokens for burning or storage)
    ///
    /// Used as intermediate account when burning token_in or as permanent storage
    /// when program lacks mint authority.
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_in_program
    )]
    pub vault_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Redemption vault account for token_out distribution when using transfer mechanism
    ///
    /// Source of output tokens when the program lacks mint authority
    /// and must transfer from pre-funded vault instead of minting.
    #[account(
        mut,
        associated_token::mint = token_out_mint,
        associated_token::authority = redemption_vault_authority,
        associated_token::token_program = token_out_program
    )]
    pub vault_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Input token mint (typically ONyc)
    ///
    /// Must be mutable to allow burning operations when program has mint authority.
    #[account(
        mut,
        constraint = token_in_mint.key() == redemption_offer.load()?.token_in_mint
            @ FulfillRedemptionRequestErrorCode::InvalidTokenInMint
    )]
    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for input token operations
    pub token_in_program: Interface<'info, TokenInterface>,

    /// Output token mint (typically stablecoin like USDC)
    ///
    /// Must be mutable to allow minting operations when program has mint authority.
    #[account(
        mut,
        constraint = token_out_mint.key() == redemption_offer.load()?.token_out_mint
            @ FulfillRedemptionRequestErrorCode::InvalidTokenOutMint
    )]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for output token operations
    pub token_out_program: Interface<'info, TokenInterface>,

    /// User's input token account
    ///
    /// Included for validation but not used in transfer since tokens are already locked in vault.
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = redeemer,
        associated_token::token_program = token_in_program
    )]
    pub user_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// User's output token account (destination for redeemed tokens)
    ///
    /// Created automatically if it doesn't exist.
    #[account(
        init_if_needed,
        payer = redemption_admin,
        associated_token::mint = token_out_mint,
        associated_token::authority = redeemer,
        associated_token::token_program = token_out_program
    )]
    pub user_token_out_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Boss's input token account for receiving tokens when program lacks mint authority
    ///
    /// Only used when token_in is ONyc and program doesn't have mint authority.
    #[account(
        mut,
        associated_token::mint = token_in_mint,
        associated_token::authority = boss,
        associated_token::token_program = token_in_program
    )]
    pub boss_token_in_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Program-derived mint authority for direct token minting
    ///
    /// Used when the program has mint authority and can mint token_out directly.
    /// CHECK: PDA derivation is validated through seeds constraint
    #[account(
        seeds = [seeds::MINT_AUTHORITY],
        bump
    )]
    pub mint_authority: AccountInfo<'info>,

    /// The user who created the redemption request
    /// CHECK: Validated against redemption_request.redeemer
    #[account(constraint = redeemer.key() == redemption_request.load()?.redeemer
        @ FulfillRedemptionRequestErrorCode::InvalidRedeemer)]
    pub redeemer: UncheckedAccount<'info>,

    /// Redemption admin must sign to authorize fulfillment
    #[account(
        mut,
        constraint = redemption_admin.key() == state.redemption_admin
            @ FulfillRedemptionRequestErrorCode::Unauthorized
    )]
    pub redemption_admin: Signer<'info>,

    /// Associated Token Program for automatic token account creation
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program required for account creation
    pub system_program: Program<'info, System>,
}

/// Fulfills a redemption request
///
/// This instruction fulfills a pending redemption request by:
/// 1. Getting the current price from the underlying offer (inverse calculation)
/// 2. Calculating token_out amount based on token_in and current price
/// 3. If token_in is ONyc and program has mint authority: burn it from vault
/// 4. If token_in is ONyc and program lacks mint authority: send to boss from vault
/// 5. If token_out program has mint authority: mint token_out to user
/// 6. If token_out program lacks mint authority: transfer from vault to user
/// 7. Update redemption request status and offer statistics
///
/// Note: token_in is already locked in the vault from create_redemption_request
///
/// # Arguments
/// * `ctx` - The instruction context containing validated accounts
///
/// # Returns
/// * `Ok(())` - If the redemption is successfully fulfilled
/// * `Err(_)` - If validation fails or token operations fail
///
/// # Access Control
/// - Only redemption_admin can fulfill redemptions
/// - Kill switch prevents fulfillment when activated
/// - Request must be pending (status == 0) and not expired
///
/// # Effects
/// - Marks redemption request as fulfilled (status = 1)
/// - Updates executed_redemptions and requested_redemptions in RedemptionOffer
/// - Burns or transfers token_in based on mint authority
/// - Mints or transfers token_out to user
///
/// # Events
/// * `RedemptionRequestFulfilledEvent` - Emitted with fulfillment details
pub fn fulfill_redemption_request(ctx: Context<FulfillRedemptionRequest>) -> Result<()> {
    let redemption_request = ctx.accounts.redemption_request.load()?;
    let token_in_amount = redemption_request.amount;
    drop(redemption_request);

    // Get current price from the offer and calculate token_out amount
    // For redemption: token_in is ONyc (offer's token_out), token_out is stable (offer's token_in)
    // So we use inverse calculation: token_out = token_in * price
    let offer = ctx.accounts.offer.load()?;
    let result = process_offer_core(
        &offer,
        token_in_amount,
        &ctx.accounts.token_out_mint, // Inverted: offer's token_in is redemption's token_out
        &ctx.accounts.token_in_mint,  // Inverted: offer's token_out is redemption's token_in
    )?;

    // For redemption, the token_out_amount from the offer calculation is what user receives
    let token_out_amount = result.token_out_amount;
    drop(offer);

    // Token_in is already locked in the vault from create_redemption_request
    // Now we burn or transfer it based on mint authority

    let is_onyc = ctx.accounts.token_in_mint.key() == ctx.accounts.state.onyc_mint;
    let has_mint_authority = ctx.accounts.token_in_mint.mint_authority
        .as_ref()
        .map(|auth| auth == &ctx.accounts.mint_authority.key())
        .unwrap_or(false);

    if is_onyc && has_mint_authority {
        // Burn token_in from vault
        let redemption_vault_authority_bump = ctx.bumps.redemption_vault_authority;
        let signer_seeds: &[&[&[u8]]] = &[&[
            seeds::REDEMPTION_OFFER_VAULT_AUTHORITY,
            &[redemption_vault_authority_bump],
        ]];

        burn(
            CpiContext::new_with_signer(
                ctx.accounts.token_in_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.token_in_mint.to_account_info(),
                    from: ctx.accounts.vault_token_in_account.to_account_info(),
                    authority: ctx.accounts.redemption_vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            token_in_amount,
        )?;
    } else {
        let redemption_vault_authority_bump = ctx.bumps.redemption_vault_authority;
        let signer_seeds: &[&[&[u8]]] = &[&[
            seeds::REDEMPTION_OFFER_VAULT_AUTHORITY,
            &[redemption_vault_authority_bump],
        ]];

        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_in_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_in_account.to_account_info(),
                    to: ctx.accounts.boss_token_in_account.to_account_info(),
                    authority: ctx.accounts.redemption_vault_authority.to_account_info(),
                    mint: ctx.accounts.token_in_mint.to_account_info(),
                },
                signer_seeds,
            ),
            token_in_amount,
            ctx.accounts.token_in_mint.decimals,
        )?;
    }

    let has_token_out_mint_authority = ctx.accounts.token_out_mint.mint_authority
        .as_ref()
        .map(|auth| auth == &ctx.accounts.mint_authority.key())
        .unwrap_or(false);

    if has_token_out_mint_authority {
        // Mint token_out directly to user
        let mint_authority_bump = ctx.bumps.mint_authority;
        let signer_seeds: &[&[&[u8]]] = &[&[seeds::MINT_AUTHORITY, &[mint_authority_bump]]];

        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_out_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.token_out_mint.to_account_info(),
                    to: ctx.accounts.user_token_out_account.to_account_info(),
                    authority: ctx.accounts.mint_authority.to_account_info(),
                },
                signer_seeds,
            ),
            token_out_amount,
        )?;
    } else {
        // Transfer token_out from vault to user
        let redemption_vault_authority_bump = ctx.bumps.redemption_vault_authority;
        let signer_seeds: &[&[&[u8]]] = &[&[
            seeds::REDEMPTION_OFFER_VAULT_AUTHORITY,
            &[redemption_vault_authority_bump],
        ]];

        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_out_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_out_account.to_account_info(),
                    to: ctx.accounts.user_token_out_account.to_account_info(),
                    authority: ctx.accounts.redemption_vault_authority.to_account_info(),
                    mint: ctx.accounts.token_out_mint.to_account_info(),
                },
                signer_seeds,
            ),
            token_out_amount,
            ctx.accounts.token_out_mint.decimals,
        )?;
    }

    let mut redemption_request = ctx.accounts.redemption_request.load_mut()?;
    redemption_request.status = 1;

    let mut redemption_offer = ctx.accounts.redemption_offer.load_mut()?;
    redemption_offer.executed_redemptions = redemption_offer
        .executed_redemptions
        .checked_add(token_in_amount as u128)
        .ok_or(FulfillRedemptionRequestErrorCode::ArithmeticOverflow)?;
    redemption_offer.requested_redemptions = redemption_offer
        .requested_redemptions
        .checked_sub(token_in_amount)
        .ok_or(FulfillRedemptionRequestErrorCode::ArithmeticOverflow)?;

    msg!(
        "Redemption request fulfilled: request={}, token_in={}, token_out={}, price={}, redeemer={}",
        ctx.accounts.redemption_request.key(),
        token_in_amount,
        token_out_amount,
        result.current_price,
        ctx.accounts.redeemer.key()
    );

    emit!(RedemptionRequestFulfilledEvent {
        redemption_request_pda: ctx.accounts.redemption_request.key(),
        redemption_offer: ctx.accounts.redemption_offer.key(),
        redeemer: ctx.accounts.redeemer.key(),
        token_in_amount,
        token_out_amount,
        current_price: result.current_price,
    });

    Ok(())
}

/// Error codes for redemption fulfillment operations
#[error_code]
pub enum FulfillRedemptionRequestErrorCode {
    /// Caller is not authorized (redemption_admin mismatch)
    #[msg("Unauthorized: redemption_admin signature required")]
    Unauthorized,

    /// The boss account does not match the one stored in program state
    #[msg("Invalid boss account")]
    InvalidBoss,

    /// The program kill switch is activated
    #[msg("Kill switch is activated")]
    KillSwitchActivated,

    /// Redemption request has already been processed
    #[msg("Redemption request has already been processed")]
    RequestAlreadyProcessed,

    /// Redemption offer mismatch
    #[msg("Redemption offer does not match request")]
    OfferMismatch,

    /// Offer mint configuration mismatch
    #[msg("Offer mints do not match redemption offer (inverted) mints")]
    OfferMintMismatch,

    /// Invalid token_in mint
    #[msg("Invalid token_in mint")]
    InvalidTokenInMint,

    /// Invalid token_out mint
    #[msg("Invalid token_out mint")]
    InvalidTokenOutMint,

    /// Invalid redeemer
    #[msg("Redeemer does not match redemption request")]
    InvalidRedeemer,

    /// Arithmetic overflow occurred
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
}
