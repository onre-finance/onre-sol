use crate::{
    model::{OfferDirection, VaultConfig},
    utils::vault_authority::validate_liquidity_vault,
    OnreError, ID,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::{self, TokenAccount, TransferChecked},
};

use super::TakeOffer;

#[derive(Default)]
pub(super) struct LiquidityContext {
    pub balance: u64,
    pub refill_target_bps: u16,
}

/// Only the explicitly selected vault contributes payout liquidity.
pub(super) fn validate(program_id: &Pubkey, a: &TakeOffer) -> Result<LiquidityContext> {
    if a.offer_config.liquidity_vault == Pubkey::default() {
        require!(
            a.offer_config.direction == OfferDirection::AssetToManaged,
            OnreError::LiquidityVaultRequired
        );
        require!(
            a.liquidity_vault.is_none() && a.liquidity_token.is_none(),
            OnreError::InvalidConfigurableVault
        );
        return Ok(LiquidityContext::default());
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
    let balance = balance(a, token, &vault.key())?;
    let data = vault.try_borrow_data()?;
    let refill_target_bps = if data.starts_with(VaultConfig::DISCRIMINATOR) {
        VaultConfig::try_deserialize(&mut data.as_ref())?.refill_target_bps
    } else {
        0
    };
    require!(
        refill_target_bps <= crate::constants::MAX_BASIS_POINTS,
        OnreError::InvalidAmount
    );
    Ok(LiquidityContext {
        balance,
        refill_target_bps,
    })
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

/// Fund only the selected asset ATA. Validation already checked its canonical
/// address; initialize it lazily only when a positive refill is actually needed.
pub(super) fn refill<'info>(
    a: &TakeOffer<'info>,
    input: &InterfaceAccount<'info, TokenAccount>,
    input_authority: &AccountInfo<'info>,
    input_seeds: Option<&[&[&[u8]]]>,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let vault = a
        .liquidity_vault
        .as_ref()
        .ok_or(OnreError::LiquidityVaultRequired)?;
    let token = a
        .liquidity_token
        .as_ref()
        .ok_or(OnreError::InvalidConfigurableVaultTokenAccount)?;
    if *token.owner == System::id() {
        anchor_spl::associated_token::create_idempotent(CpiContext::new(
            a.associated_token_program.key(),
            anchor_spl::associated_token::Create {
                payer: a.user.to_account_info(),
                associated_token: token.to_account_info(),
                authority: vault.to_account_info(),
                mint: a.token_in_mint.to_account_info(),
                system_program: a.system_program.to_account_info(),
                token_program: a.token_in_program.to_account_info(),
            },
        ))?;
    }
    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            a.token_in_program.key(),
            TransferChecked {
                mint: a.token_in_mint.to_account_info(),
                from: input.to_account_info(),
                to: token.to_account_info(),
                authority: input_authority.clone(),
            },
            input_seeds.unwrap_or(&[]),
        ),
        amount,
        a.token_in_mint.decimals,
    )
}
