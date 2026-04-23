use crate::constants::seeds;
use crate::instructions::offer::process_offer_core;
use crate::instructions::redemption::{process_redemption_core, RedemptionOffer};
use crate::instructions::Offer;
use crate::state::State;
use anchor_lang::solana_program::program::set_return_data;
use anchor_lang::{prelude::*, system_program, Accounts, AnchorDeserialize, AnchorSerialize};
use anchor_spl::token_interface::Mint;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwapSide {
    Buy,
    Sell,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SwapQuote {
    pub offer: Pubkey,
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub token_in_amount: u64,
    pub token_in_net_amount: u64,
    pub token_in_fee_amount: u64,
    pub token_out_amount: u64,
    pub minimum_out: u64,
    pub current_price: u64,
    pub quoted_at: i64,
}

struct SwapQuoteComputation {
    current_price: u64,
    token_in_net_amount: u64,
    token_in_fee_amount: u64,
    token_out_amount: u64,
}

#[derive(Accounts)]
pub struct QuoteSwapBuy<'info> {
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = state.is_killed == false @ crate::OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,
}

#[derive(Accounts)]
pub struct QuoteSwapSell<'info> {
    pub offer: AccountLoader<'info, Offer>,

    #[account(
        seeds = [
            seeds::REDEMPTION_OFFER,
            token_in_mint.key().as_ref(),
            token_out_mint.key().as_ref()
        ],
        bump
    )]
    /// CHECK: PDA address is validated by seeds; data is optional and loaded in instruction logic.
    pub redemption_offer: UncheckedAccount<'info>,

    #[account(
        seeds = [seeds::STATE],
        bump = state.bump,
        constraint = state.is_killed == false @ crate::OnreError::KillSwitchActivated
    )]
    pub state: Box<Account<'info, State>>,

    pub token_in_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_out_mint: Box<InterfaceAccount<'info, Mint>>,
}

pub(crate) fn resolve_swap_side(
    state: &State,
    token_in_mint: Pubkey,
    token_out_mint: Pubkey,
) -> Result<(SwapSide, Pubkey)> {
    require!(
        token_in_mint != token_out_mint,
        crate::OnreError::InvalidSwapPair
    );

    if token_out_mint == state.onyc_mint && token_in_mint != state.onyc_mint {
        return Ok((SwapSide::Buy, token_in_mint));
    }

    if token_in_mint == state.onyc_mint && token_out_mint != state.onyc_mint {
        return Ok((SwapSide::Sell, token_out_mint));
    }

    Err(error!(crate::OnreError::InvalidSwapPair))
}

pub(crate) fn validate_canonical_offer(
    program_id: &Pubkey,
    state: &State,
    offer_key: Pubkey,
    token_in_mint: Pubkey,
    token_out_mint: Pubkey,
) -> Result<SwapSide> {
    let (side, asset_mint) = resolve_swap_side(state, token_in_mint, token_out_mint)?;
    let (expected_offer, _) = Pubkey::find_program_address(
        &[seeds::OFFER, asset_mint.as_ref(), state.onyc_mint.as_ref()],
        program_id,
    );
    require_keys_eq!(offer_key, expected_offer, crate::OnreError::OfferMismatch);
    Ok(side)
}

pub(crate) fn redemption_offer_fee_basis_points(
    program_id: &Pubkey,
    redemption_offer_account: &UncheckedAccount,
    offer_key: Pubkey,
    token_in_mint: Pubkey,
    token_out_mint: Pubkey,
) -> Result<u16> {
    let account_info = redemption_offer_account.to_account_info();
    if account_info.owner == &system_program::ID {
        require!(
            account_info.data_is_empty(),
            crate::OnreError::InvalidRedemptionOfferData
        );
        return Ok(0);
    }

    require_keys_eq!(
        *account_info.owner,
        *program_id,
        crate::OnreError::InvalidRedemptionOfferOwner
    );

    let data = account_info.try_borrow_data()?;
    let mut slice: &[u8] = &data;
    let redemption_offer = RedemptionOffer::try_deserialize(&mut slice)
        .map_err(|_| error!(crate::OnreError::InvalidRedemptionOfferData))?;

    require_keys_eq!(
        redemption_offer.offer,
        offer_key,
        crate::OnreError::InvalidRedemptionOffer
    );
    require_keys_eq!(
        redemption_offer.token_in_mint,
        token_in_mint,
        crate::OnreError::InvalidRedemptionOffer
    );
    require_keys_eq!(
        redemption_offer.token_out_mint,
        token_out_mint,
        crate::OnreError::InvalidRedemptionOffer
    );

    Ok(redemption_offer.fee_basis_points)
}

pub fn build_swap_buy_quote(
    program_id: &Pubkey,
    state: &State,
    offer_key: Pubkey,
    offer: &Offer,
    token_in_amount: u64,
    token_in_mint: &InterfaceAccount<Mint>,
    token_out_mint: &InterfaceAccount<Mint>,
) -> Result<SwapQuote> {
    let quoted_at = Clock::get()?.unix_timestamp;

    let side = validate_canonical_offer(
        program_id,
        state,
        offer_key,
        token_in_mint.key(),
        token_out_mint.key(),
    )?;
    require!(side == SwapSide::Buy, crate::OnreError::InvalidSwapPair);

    let result = process_offer_core(offer, token_in_amount, token_in_mint, token_out_mint).map(
        |result| SwapQuoteComputation {
            current_price: result.current_price,
            token_in_net_amount: result.token_in_net_amount,
            token_in_fee_amount: result.token_in_fee_amount,
            token_out_amount: result.token_out_amount,
        },
    )?;

    Ok(SwapQuote {
        offer: offer_key,
        token_in_mint: token_in_mint.key(),
        token_out_mint: token_out_mint.key(),
        token_in_amount,
        token_in_net_amount: result.token_in_net_amount,
        token_in_fee_amount: result.token_in_fee_amount,
        token_out_amount: result.token_out_amount,
        minimum_out: result.token_out_amount,
        current_price: result.current_price,
        quoted_at,
    })
}

pub fn build_swap_sell_quote(
    program_id: &Pubkey,
    state: &State,
    offer_key: Pubkey,
    offer: &Offer,
    redemption_fee_basis_points: u16,
    token_in_amount: u64,
    token_in_mint: &InterfaceAccount<Mint>,
    token_out_mint: &InterfaceAccount<Mint>,
) -> Result<SwapQuote> {
    let quoted_at = Clock::get()?.unix_timestamp;

    let side = validate_canonical_offer(
        program_id,
        state,
        offer_key,
        token_in_mint.key(),
        token_out_mint.key(),
    )?;
    require!(side == SwapSide::Sell, crate::OnreError::InvalidSwapPair);

    let result = process_redemption_core(
        offer,
        token_in_amount,
        token_in_mint,
        token_out_mint,
        redemption_fee_basis_points,
    )
    .map(|result| SwapQuoteComputation {
        current_price: result.price,
        token_in_net_amount: result.token_in_net_amount,
        token_in_fee_amount: result.token_in_fee_amount,
        token_out_amount: result.token_out_amount,
    })?;

    Ok(SwapQuote {
        offer: offer_key,
        token_in_mint: token_in_mint.key(),
        token_out_mint: token_out_mint.key(),
        token_in_amount,
        token_in_net_amount: result.token_in_net_amount,
        token_in_fee_amount: result.token_in_fee_amount,
        token_out_amount: result.token_out_amount,
        minimum_out: result.token_out_amount,
        current_price: result.current_price,
        quoted_at,
    })
}

pub fn quote_swap_buy(ctx: Context<QuoteSwapBuy>, token_in_amount: u64) -> Result<()> {
    let offer = ctx.accounts.offer.load()?;
    let quote = build_swap_buy_quote(
        ctx.program_id,
        &ctx.accounts.state,
        ctx.accounts.offer.key(),
        &offer,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
    )?;
    let mut serialized_quote = Vec::new();
    quote.serialize(&mut serialized_quote)?;
    set_return_data(&serialized_quote);

    msg!(
        "Swap buy quote - offer: {}, token_in: {}, minimum_out: {}",
        quote.offer,
        quote.token_in_amount,
        quote.minimum_out
    );

    Ok(())
}

pub fn quote_swap_sell(ctx: Context<QuoteSwapSell>, token_in_amount: u64) -> Result<()> {
    let offer = ctx.accounts.offer.load()?;
    let redemption_fee_basis_points = redemption_offer_fee_basis_points(
        ctx.program_id,
        &ctx.accounts.redemption_offer,
        ctx.accounts.offer.key(),
        ctx.accounts.token_in_mint.key(),
        ctx.accounts.token_out_mint.key(),
    )?;
    let quote = build_swap_sell_quote(
        ctx.program_id,
        &ctx.accounts.state,
        ctx.accounts.offer.key(),
        &offer,
        redemption_fee_basis_points,
        token_in_amount,
        &ctx.accounts.token_in_mint,
        &ctx.accounts.token_out_mint,
    )?;
    let mut serialized_quote = Vec::new();
    quote.serialize(&mut serialized_quote)?;
    set_return_data(&serialized_quote);

    msg!(
        "Swap sell quote - offer: {}, redemption_offer: {}, token_in: {}, minimum_out: {}",
        quote.offer,
        ctx.accounts.redemption_offer.key(),
        quote.token_in_amount,
        quote.minimum_out
    );

    Ok(())
}
