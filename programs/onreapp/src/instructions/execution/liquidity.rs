use super::TakeOffer;
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

#[derive(Default)]
pub(super) struct VaultContext {
    pub balance: u64,
    pub refill_target_bps: u16,
}

pub(super) struct LiquidityContext {
    pub source: VaultContext,
    pub destination: VaultContext,
}

/// Validate each configured role against its own mint and canonical ATA. Missing
/// inventory never falls back to minting; only an absent buy source permits minting.
pub(super) fn validate(program_id: &Pubkey, a: &TakeOffer) -> Result<LiquidityContext> {
    require!(
        a.offer_config.direction != OfferDirection::ManagedToAsset
            || a.offer_config.source_vault != Pubkey::default(),
        OnreError::LiquidityVaultRequired
    );
    Ok(LiquidityContext {
        source: validate_role(
            program_id,
            a.offer_config.source_vault,
            a.source_vault.as_ref(),
            a.source_token_out.as_ref(),
            a.token_out_mint.key(),
            a.token_out_program.key(),
        )?,
        destination: validate_role(
            program_id,
            a.offer_config.destination_vault,
            a.destination_vault.as_ref(),
            a.destination_token_in.as_ref(),
            a.token_in_mint.key(),
            a.token_in_program.key(),
        )?,
    })
}

fn validate_role(
    program_id: &Pubkey,
    reference: Pubkey,
    vault: Option<&UncheckedAccount>,
    token: Option<&UncheckedAccount>,
    mint: Pubkey,
    program: Pubkey,
) -> Result<VaultContext> {
    if reference == Pubkey::default() {
        require!(
            vault.is_none() && token.is_none(),
            OnreError::InvalidConfigurableVault
        );
        return Ok(VaultContext::default());
    }
    let vault = vault.ok_or(OnreError::LiquidityVaultRequired)?;
    require_keys_eq!(vault.key(), reference, OnreError::InvalidConfigurableVault);
    validate_liquidity_vault(&vault.to_account_info(), program_id)?;
    let token = token.ok_or(OnreError::InvalidConfigurableVaultTokenAccount)?;
    require_keys_eq!(
        token.key(),
        get_associated_token_address_with_program_id(&reference, &mint, &program),
        OnreError::InvalidConfigurableVaultTokenAccount
    );
    let balance = if token.data_is_empty() {
        require_keys_eq!(
            *token.owner,
            System::id(),
            OnreError::InvalidConfigurableVaultTokenAccount
        );
        0
    } else {
        require_keys_eq!(*token.owner, program, OnreError::InvalidTokenProgram);
        let data = token.try_borrow_data()?;
        let value = TokenAccount::try_deserialize(&mut data.as_ref())?;
        require!(
            value.mint == mint && value.owner == reference,
            OnreError::InvalidConfigurableVaultTokenAccount
        );
        value.amount
    };
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
    Ok(VaultContext {
        balance,
        refill_target_bps,
    })
}

/// Pay output exclusively from the configured source, using its validated seeds.
pub(super) fn consume<'info>(
    a: &TakeOffer<'info>,
    amount: u64,
    destination: &InterfaceAccount<'info, TokenAccount>,
) -> Result<()> {
    let vault = a
        .source_vault
        .as_ref()
        .ok_or(OnreError::LiquidityVaultRequired)?;
    let token = a
        .source_token_out
        .as_ref()
        .ok_or(OnreError::InvalidConfigurableVaultTokenAccount)?;
    // BUFFER can mint into a selected legacy fee vault after quote validation.
    // Read its actual post-accrual inventory rather than a cached pre-accrual balance.
    let balance = if token.data_is_empty() {
        0
    } else {
        TokenAccount::try_deserialize(&mut token.try_borrow_data()?.as_ref())?.amount
    };
    require!(amount <= balance, OnreError::InsufficientBalance);
    let authority = validate_liquidity_vault(&vault.to_account_info(), &ID)?;
    let seeds = authority.signer_seeds();
    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            a.token_out_program.key(),
            TransferChecked {
                mint: a.token_out_mint.to_account_info(),
                from: token.to_account_info(),
                to: destination.to_account_info(),
                authority: vault.to_account_info(),
            },
            &[&seeds],
        ),
        amount,
        a.token_out_mint.decimals,
    )
}

/// Deposit input into the exact configured destination. Buys call with the TVL
/// refill deficit; sells call with the full net managed input instead of burning.
pub(super) fn deposit_destination<'info>(
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
        .destination_vault
        .as_ref()
        .ok_or(OnreError::LiquidityVaultRequired)?;
    let token = a
        .destination_token_in
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
