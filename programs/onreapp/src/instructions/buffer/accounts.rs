use crate::constants::seeds;
use crate::instructions::buffer::{BufferErrorCode, BufferState};
use crate::utils::{load_pda_account, store_pda_account};
use anchor_lang::{prelude::*, Accounts};
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token_interface::{Mint, TokenInterface};

#[derive(Accounts)]
pub struct BufferAccrualAccounts<'info> {
    /// CHECK: Parsed and validated only when ONyc accrual is required.
    #[account(mut)]
    pub buffer_state: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against the expected buffer ATA.
    #[account(mut)]
    pub buffer_vault_onyc_account: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against the expected management fee ATA.
    #[account(mut)]
    pub management_fee_vault_onyc_account: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against the expected performance fee ATA.
    #[account(mut)]
    pub performance_fee_vault_onyc_account: UncheckedAccount<'info>,
}

impl<'info> BufferAccrualAccounts<'info> {
    pub fn is_initialized(&self) -> bool {
        self.try_load_buffer_state().is_ok()
    }

    fn try_load_buffer_state(&self) -> Result<BufferState> {
        load_pda_account(
            &self.buffer_state,
            &crate::ID,
            BufferErrorCode::InvalidOnycMint.into(),
            BufferErrorCode::InvalidOnycMint.into(),
        )
    }

    pub fn load_buffer_state(&self) -> Result<BufferState> {
        self.try_load_buffer_state()
    }

    pub fn buffer_vault_onyc_account_info(&self) -> AccountInfo<'info> {
        self.buffer_vault_onyc_account.to_account_info()
    }

    pub fn management_fee_vault_onyc_account_info(&self) -> AccountInfo<'info> {
        self.management_fee_vault_onyc_account.to_account_info()
    }

    pub fn performance_fee_vault_onyc_account_info(&self) -> AccountInfo<'info> {
        self.performance_fee_vault_onyc_account.to_account_info()
    }

    pub fn store_buffer_state(&self, buffer_state: &BufferState) -> Result<()> {
        store_pda_account(&self.buffer_state, buffer_state)
    }
}

pub fn validate_buffer_onyc_vault_accounts<'info>(
    program_id: &Pubkey,
    buffer_state: &BufferState,
    buffer_vault_onyc_account: &AccountInfo<'info>,
    management_fee_vault_onyc_account: &AccountInfo<'info>,
    performance_fee_vault_onyc_account: &AccountInfo<'info>,
    onyc_mint: &InterfaceAccount<'info, Mint>,
    token_program: &Interface<'info, TokenInterface>,
) -> Result<()> {
    let expected_buffer_vault_onyc_account = get_associated_token_address_with_program_id(
        &Pubkey::find_program_address(&[seeds::RESERVE_VAULT_AUTHORITY], program_id).0,
        &onyc_mint.key(),
        &token_program.key(),
    );
    let expected_management_fee_vault_onyc_account = get_associated_token_address_with_program_id(
        &Pubkey::find_program_address(&[seeds::MANAGEMENT_FEE_VAULT_AUTHORITY], program_id).0,
        &onyc_mint.key(),
        &token_program.key(),
    );
    let expected_performance_fee_vault_onyc_account = get_associated_token_address_with_program_id(
        &Pubkey::find_program_address(&[seeds::PERFORMANCE_FEE_VAULT_AUTHORITY], program_id).0,
        &onyc_mint.key(),
        &token_program.key(),
    );

    require_keys_eq!(
        buffer_state.onyc_mint,
        onyc_mint.key(),
        BufferErrorCode::InvalidOnycMint
    );
    require_keys_eq!(
        buffer_vault_onyc_account.key(),
        expected_buffer_vault_onyc_account,
        BufferErrorCode::InvalidOnycMint
    );
    require_keys_eq!(
        management_fee_vault_onyc_account.key(),
        expected_management_fee_vault_onyc_account,
        BufferErrorCode::InvalidOnycMint
    );
    require_keys_eq!(
        performance_fee_vault_onyc_account.key(),
        expected_performance_fee_vault_onyc_account,
        BufferErrorCode::InvalidOnycMint
    );

    Ok(())
}
