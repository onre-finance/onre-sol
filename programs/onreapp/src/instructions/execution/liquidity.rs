use crate::{
    model::OfferDirection, utils::vault_authority::validate_liquidity_vault, OnreError, ID,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::{self, TokenAccount, TransferChecked},
};

use super::TakeOffer;

/// Only the explicitly selected vault contributes payout liquidity.
pub(super) fn validate(program_id: &Pubkey, a: &TakeOffer) -> Result<u64> {
    if a.offer_config.liquidity_vault == Pubkey::default() {
        require!(
            a.offer_config.direction == OfferDirection::AssetToManaged,
            OnreError::LiquidityVaultRequired
        );
        require!(
            a.liquidity_vault.is_none() && a.liquidity_token.is_none(),
            OnreError::InvalidConfigurableVault
        );
        return Ok(0);
    }
    let vault = a
        .liquidity_vault
        .as_ref()
        .ok_or(OnreError::LiquidityVaultRequired)?;
    require_keys_eq!(
        vault.key(),
        a.offer_config.liquidity_vault,
        OnreError::InvalidConfigurableVault
    );
    validate_liquidity_vault(&vault.to_account_info(), program_id)?;
    let token = a
        .liquidity_token
        .as_ref()
        .ok_or(OnreError::InvalidConfigurableVaultTokenAccount)?;
    balance(a, token, &vault.key())
}

fn balance(a: &TakeOffer, token: &UncheckedAccount, authority: &Pubkey) -> Result<u64> {
    let mint = a.asset_mint().key();
    let program = a.asset_program().key();
    require_keys_eq!(
        token.key(),
        get_associated_token_address_with_program_id(authority, &mint, &program),
        OnreError::InvalidConfigurableVaultTokenAccount
    );
    if token.data_is_empty() {
        require_keys_eq!(
            *token.owner,
            System::id(),
            OnreError::InvalidConfigurableVaultTokenAccount
        );
        return Ok(0);
    }
    require_keys_eq!(*token.owner, program, OnreError::InvalidTokenProgram);
    let data = token.try_borrow_data()?;
    let value = TokenAccount::try_deserialize(&mut data.as_ref())?;
    require!(
        value.mint == mint && value.owner == *authority,
        OnreError::InvalidConfigurableVaultTokenAccount
    );
    Ok(value.amount)
}

pub(super) fn consume<'info>(
    a: &TakeOffer<'info>,
    balance: u64,
    amount: u64,
    destination: &InterfaceAccount<'info, TokenAccount>,
) -> Result<()> {
    require!(amount <= balance, OnreError::InsufficientBalance);
    let vault = a
        .liquidity_vault
        .as_ref()
        .ok_or(OnreError::LiquidityVaultRequired)?;
    let token = a
        .liquidity_token
        .as_ref()
        .ok_or(OnreError::InvalidConfigurableVaultTokenAccount)?;
    let authority = validate_liquidity_vault(&vault.to_account_info(), &ID)?;
    let signer_seeds = authority.signer_seeds();
    transfer(
        a,
        token,
        destination,
        vault.to_account_info(),
        &signer_seeds,
        amount,
    )
}

fn transfer<'info>(
    a: &TakeOffer<'info>,
    token: &UncheckedAccount<'info>,
    destination: &InterfaceAccount<'info, TokenAccount>,
    authority: AccountInfo<'info>,
    seeds: &[&[u8]],
    amount: u64,
) -> Result<()> {
    let accounts = TransferChecked {
        mint: a.token_out_mint.to_account_info(),
        from: token.to_account_info(),
        to: destination.to_account_info(),
        authority,
    };
    token_interface::transfer_checked(
        CpiContext::new_with_signer(a.token_out_program.key(), accounts, &[seeds]),
        amount,
        a.token_out_mint.decimals,
    )
}
