use crate::instructions::configurable_vault::ConfigurableVaultInit;
use crate::utils::{
    get_or_create_associated_token_account, load_or_init_pda_account, store_pda_account,
    EnsureAtaParams, PdaAccountInit,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;

pub(crate) struct ConfigurableVaultTokenAccountParams<'a, 'info> {
    pub vault: &'info UncheckedAccount<'info>,
    pub token_account: &'info UncheckedAccount<'info>,
    pub payer: AccountInfo<'info>,
    pub mint_account: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub associated_token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub program_id: &'a Pubkey,
}

pub(crate) fn get_or_create_configurable_vault_token_account<'info, const KIND: u8>(
    params: ConfigurableVaultTokenAccountParams<'_, 'info>,
) -> Result<InterfaceAccount<'info, TokenAccount>> {
    let vault_info = params.vault.to_account_info();
    let (expected_vault, vault_bump) = Pubkey::find_program_address(
        ConfigurableVaultInit::<KIND>::pda_seed_prefixes(),
        params.program_id,
    );
    require_keys_eq!(
        expected_vault,
        params.vault.key(),
        crate::OnreError::InvalidConfigurableVault
    );

    let vault_is_zeroed = vault_info.try_borrow_data()?.iter().all(|byte| *byte == 0);
    let should_store_vault = vault_info.owner == &system_program::ID || vault_is_zeroed;
    let vault = if vault_info.owner == params.program_id && vault_is_zeroed {
        ConfigurableVaultInit::<KIND>::init_value(vault_bump)
    } else {
        load_or_init_pda_account::<ConfigurableVaultInit<KIND>>(
            &vault_info,
            &params.payer,
            &params.system_program,
            params.program_id,
            vault_bump,
        )?
    };
    if should_store_vault {
        store_pda_account(&vault_info, &vault)?;
    }

    get_or_create_associated_token_account(EnsureAtaParams {
        ata_account: params.token_account,
        payer: params.payer,
        authority_account: vault_info,
        mint_account: params.mint_account.clone(),
        token_program: params.token_program.clone(),
        associated_token_program: params.associated_token_program,
        system_program: params.system_program,
        authority: params.vault.key(),
        mint: params.mint_account.key(),
        token_program_id: params.token_program.key(),
        invalid_account_error: crate::OnreError::InvalidConfigurableVaultTokenAccount,
    })
}
