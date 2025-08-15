use crate::state::Offer;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct GetCirculatingSupply<'info> {
    /// The offer to read circulating supply from
    pub offer: Account<'info, Offer>,

    #[account(
        associated_token::mint = token_mint,
        associated_token::authority = vault_authority,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    /// CHECK: The vault authority is an owner of the vault token account.
    #[account()]
    pub vault_authority: AccountInfo<'info>,

    #[account(
        associated_token::mint = token_mint,
        associated_token::authority = offer_token_authority.key(),
    )]
    pub offer_token_account: Account<'info, TokenAccount>,

    /// # Note
    /// This account is marked with `CHECK` as it's validated by the seed derivation.
    #[account(
        seeds = [b"offer_authority", offer.offer_id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer_token_authority: AccountInfo<'info>,

    pub token_mint: Account<'info, Mint>,

    /// SPL Token program for token operations.
    pub token_program: Program<'info, Token>,

    /// Solana System program for account creation and rent payment.
    pub system_program: Program<'info, System>,
}

pub fn get_circulating_supply(ctx: Context<GetCirculatingSupply>) -> Result<u64> {
    let circulating_supply = &ctx.accounts.token_mint.supply;
    let vault_token_amount = ctx.accounts.vault_token_account.amount;
    let offer_token_amount = ctx.accounts.offer_token_account.amount;
    let circulating_supply = circulating_supply.checked_sub(
        vault_token_amount.checked_add(offer_token_amount).unwrap()
    ).unwrap();
    Ok(circulating_supply)
}

