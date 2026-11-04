use crate::{
    constants::seeds,
    model::{seeds::VAULT, OfferDirection, VaultKind},
    OnreError, ID,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::{self, TokenAccount, TransferChecked},
};

use super::TakeOffer;

#[derive(Default)]
pub(super) struct LiquidityBalances {
    pub legacy: u64,
    pub current: u64,
}

impl LiquidityBalances {
    pub fn total(&self) -> Result<u64> {
        self.legacy
            .checked_add(self.current)
            .ok_or_else(|| OnreError::MathOverflow.into())
    }

    /// Drain legacy liquidity first while retaining one combined solvency check.
    pub fn split(&self, amount: u64) -> Result<(u64, u64)> {
        require!(amount <= self.total()?, OnreError::InsufficientBalance);
        let legacy = amount.min(self.legacy);
        Ok((legacy, amount - legacy))
    }
}

pub(super) fn validate(program_id: &Pubkey, a: &TakeOffer) -> Result<LiquidityBalances> {
    if a.offer_config.liquidity_vault == Pubkey::default() {
        require!(
            a.offer_config.direction == OfferDirection::AssetToManaged,
            OnreError::LiquidityVaultRequired
        );
        require!(
            a.liquidity_vault.is_none()
                && a.liquidity_token.is_none()
                && a.legacy_liquidity_authority.is_none()
                && a.legacy_liquidity_token.is_none(),
            OnreError::InvalidConfigurableVault
        );
        return Ok(LiquidityBalances::default());
    }
    let vault = a
        .liquidity_vault
        .as_ref()
        .ok_or(OnreError::LiquidityVaultRequired)?;
    let (expected, bump) = Pubkey::find_program_address(
        &[
            VAULT,
            &[VaultKind::Liquidity as u8],
            &vault.instance_id.to_le_bytes(),
        ],
        program_id,
    );
    require!(
        vault.key() == a.offer_config.liquidity_vault
            && vault.key() == expected
            && vault.bump == bump,
        OnreError::InvalidConfigurableVault
    );
    require!(
        vault.kind == VaultKind::Liquidity,
        OnreError::InvalidConfigurableVaultKind
    );
    let token = a
        .liquidity_token
        .as_ref()
        .ok_or(OnreError::InvalidConfigurableVaultTokenAccount)?;
    let current = balance(a, token, &vault.key())?;
    let legacy = if a.managed_mint().key() == a.state.onyc_mint {
        let authority = a
            .legacy_liquidity_authority
            .as_ref()
            .ok_or(OnreError::InvalidConfigurableVault)?;
        let expected =
            Pubkey::find_program_address(&[seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], program_id).0;
        require_keys_eq!(
            authority.key(),
            expected,
            OnreError::InvalidConfigurableVault
        );
        let token = a
            .legacy_liquidity_token
            .as_ref()
            .ok_or(OnreError::InvalidConfigurableVaultTokenAccount)?;
        balance(a, token, &authority.key())?
    } else {
        require!(
            a.legacy_liquidity_authority.is_none() && a.legacy_liquidity_token.is_none(),
            OnreError::InvalidConfigurableVault
        );
        0
    };
    let result = LiquidityBalances { legacy, current };
    result.total()?;
    Ok(result)
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
    balances: &LiquidityBalances,
    amount: u64,
    destination: &InterfaceAccount<'info, TokenAccount>,
) -> Result<()> {
    let (legacy_amount, current_amount) = balances.split(amount)?;
    if legacy_amount > 0 {
        let authority = a
            .legacy_liquidity_authority
            .as_ref()
            .ok_or(OnreError::InvalidConfigurableVault)?;
        let token = a
            .legacy_liquidity_token
            .as_ref()
            .ok_or(OnreError::InvalidConfigurableVaultTokenAccount)?;
        let (_, bump) =
            Pubkey::find_program_address(&[seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], &ID);
        transfer(
            a,
            token,
            destination,
            authority.to_account_info(),
            &[seeds::REDEMPTION_OFFER_VAULT_AUTHORITY, &[bump]],
            legacy_amount,
        )?;
    }
    if current_amount > 0 {
        let vault = a
            .liquidity_vault
            .as_ref()
            .ok_or(OnreError::LiquidityVaultRequired)?;
        let token = a
            .liquidity_token
            .as_ref()
            .ok_or(OnreError::InvalidConfigurableVaultTokenAccount)?;
        transfer(
            a,
            token,
            destination,
            vault.to_account_info(),
            &[
                VAULT,
                &[VaultKind::Liquidity as u8],
                &vault.instance_id.to_le_bytes(),
                &[vault.bump],
            ],
            current_amount,
        )?;
    }
    Ok(())
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
