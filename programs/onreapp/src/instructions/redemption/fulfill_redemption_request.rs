use crate::constants::seeds;
use crate::instructions::redemption::{
    execute_redemption_operations, process_redemption_core, ExecuteRedemptionOpsParams,
    RedemptionOffer, RedemptionRequest,
};
use crate::instructions::Offer;
use crate::state::State;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

/// Event emitted when a redemption request is successfully fulfilled
///
/// Provides transparency for tracking redemption fulfillment and token exchange details.
#[event]
pub struct RedemptionRequestFulfilledEvent {
    /// The PDA address of the fulfilled redemption request
    pub redemption_request_pda: Pubkey,
    /// Reference to the redemption offer pda
    pub redemption_offer_pda: Pubkey,
    /// User who created the redemption request
    pub redeemer: Pubkey,
    /// Net amount of token_in tokens burned/transferred (after fees)
    pub token_in_net_amount: u64,
    /// Fee amount deducted from token_in
    pub token_in_fee_amount: u64,
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
        constraint = !state.is_killed @ FulfillRedemptionRequestErrorCode::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    /// The boss account that may receive tokens when program lacks mint authority
    /// CHECK: Account validation is enforced through state account constraint
    pub boss: UncheckedAccount<'info>,

    /// The underlying offer that defines pricing
    /// CHECK: offer address is validated through redemption_offer constraint
    pub offer: AccountLoader<'info, Offer>,

    /// The redemption offer account
    #[account(
        mut,
        constraint = redemption_offer.offer == offer.key()
            @ FulfillRedemptionRequestErrorCode::OfferMismatch
    )]
    pub redemption_offer: Box<Account<'info, RedemptionOffer>>,

    /// The redemption request account to fulfill
    /// Account is closed after fulfillment and rent is returned to redemption_admin
    #[account(
        mut,
        close = redemption_admin,
        constraint = redemption_request.offer == redemption_offer.key()
            @ FulfillRedemptionRequestErrorCode::OfferMismatch
    )]
    pub redemption_request: Box<Account<'info, RedemptionRequest>>,

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
        constraint = token_in_mint.key() == redemption_offer.token_in_mint
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
        constraint = token_out_mint.key() == redemption_offer.token_out_mint
            @ FulfillRedemptionRequestErrorCode::InvalidTokenOutMint
    )]
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token program interface for output token operations
    pub token_out_program: Interface<'info, TokenInterface>,

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
    /// Only used when program doesn't have mint authority of token_in.
    #[account(
        init_if_needed,
        payer = redemption_admin,
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
    #[account(constraint = redeemer.key() == redemption_request.redeemer
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
/// 3. If program has mint authority of token_in : burn it from vault
/// 4. If program lacks mint authority of token_int: send to boss from vault
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
    let redemption_request = &mut ctx.accounts.redemption_request;
    let token_in_amount = redemption_request.amount;

    // Use shared core processing logic for redemption
    let offer = ctx.accounts.offer.load()?;
    let result = process_redemption_core(
        &offer,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
        ctx.accounts.redemption_offer.fee_basis_points,
    )?;
    let price = result.price;
    let token_in_net_amount = result.token_in_net_amount;
    let token_in_fee_amount = result.token_in_fee_amount;
    let token_out_amount = result.token_out_amount;
    drop(offer);

    // Execute token operations (burn/transfer token_in_net, mint/transfer token_out)
    // Fee transfer is handled inside execute_redemption_operations
    execute_redemption_operations(ExecuteRedemptionOpsParams {
        token_in_program: &ctx.accounts.token_in_program,
        token_out_program: &ctx.accounts.token_out_program,
        token_in_mint: &ctx.accounts.token_in_mint,
        token_in_net_amount,
        token_in_fee_amount,
        vault_token_in_account: &ctx.accounts.vault_token_in_account,
        boss_token_in_account: &ctx.accounts.boss_token_in_account,
        redemption_vault_authority: &ctx.accounts.redemption_vault_authority,
        redemption_vault_authority_bump: ctx.bumps.redemption_vault_authority,
        token_out_mint: &ctx.accounts.token_out_mint,
        token_out_amount,
        vault_token_out_account: &ctx.accounts.vault_token_out_account,
        user_token_out_account: &ctx.accounts.user_token_out_account,
        mint_authority_pda: &ctx.accounts.mint_authority,
        mint_authority_bump: ctx.bumps.mint_authority,
        token_out_max_supply: 0, // No max supply cap for redemptions
    })?;

    let redemption_offer = &mut ctx.accounts.redemption_offer;
    redemption_offer.executed_redemptions = redemption_offer
        .executed_redemptions
        .checked_add(token_in_amount as u128)
        .ok_or(FulfillRedemptionRequestErrorCode::ArithmeticOverflow)?;

    redemption_offer.requested_redemptions = redemption_offer
        .requested_redemptions
        .checked_sub(token_in_amount as u128)
        .ok_or(FulfillRedemptionRequestErrorCode::ArithmeticUnderflow)?;

    msg!(
        "Redemption request fulfilled: request={}, token_in={} (net={}, fee={}), token_out={}, price={}, redeemer={}",
        ctx.accounts.redemption_request.key(),
        token_in_amount,
        token_in_net_amount,
        token_in_fee_amount,
        token_out_amount,
        price,
        ctx.accounts.redeemer.key()
    );

    emit!(RedemptionRequestFulfilledEvent {
        redemption_request_pda: ctx.accounts.redemption_request.key(),
        redemption_offer_pda: ctx.accounts.redemption_offer.key(),
        redeemer: ctx.accounts.redeemer.key(),
        token_in_net_amount,
        token_in_fee_amount,
        token_out_amount,
        current_price: price,
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

    /// Arithmetic underflow occurred
    #[msg("Arithmetic underflow")]
    ArithmeticUnderflow,
}
