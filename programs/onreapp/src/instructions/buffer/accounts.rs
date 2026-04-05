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
    pub reserve_vault_onyc_account: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against the expected management fee ATA.
    #[account(mut)]
    pub management_fee_vault_onyc_account: UncheckedAccount<'info>,

    /// CHECK: Validated in instruction logic against the expected performance fee ATA.
    #[account(mut)]
    pub performance_fee_vault_onyc_account: UncheckedAccount<'info>,
}

impl<'info> BufferAccrualAccounts<'info> {
    pub fn check_is_initialized(&self, program_id: &Pubkey) -> Result<bool> {
        let (expected_buffer_state, _) =
            Pubkey::find_program_address(&[seeds::BUFFER_STATE], program_id);
        require_keys_eq!(
            self.buffer_state.key(),
            expected_buffer_state,
            BufferErrorCode::InvalidBufferStateAccount
        );

        if self.buffer_state.owner != program_id || self.buffer_state.data_is_empty() {
            return Ok(false);
        }

        self.try_load_buffer_state()?;
        Ok(true)
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

    pub fn reserve_vault_onyc_account_info(&self) -> AccountInfo<'info> {
        self.reserve_vault_onyc_account.to_account_info()
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
    reserve_vault_onyc_account: &AccountInfo<'info>,
    management_fee_vault_onyc_account: &AccountInfo<'info>,
    performance_fee_vault_onyc_account: &AccountInfo<'info>,
    onyc_mint: &InterfaceAccount<'info, Mint>,
    token_program: &Interface<'info, TokenInterface>,
) -> Result<()> {
    let expected_reserve_vault_onyc_account = get_associated_token_address_with_program_id(
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
        reserve_vault_onyc_account.key(),
        expected_reserve_vault_onyc_account,
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

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::error::Error;
    use solana_program::clock::Epoch;

    fn unchecked_account_with_owner(
        key: Pubkey,
        owner: Pubkey,
        data_len: usize,
    ) -> UncheckedAccount<'static> {
        let lamports = Box::leak(Box::new(0u64));
        let data = Box::leak(vec![0u8; data_len].into_boxed_slice());
        let key_ref = Box::leak(Box::new(key));
        let owner_ref = Box::leak(Box::new(owner));
        let account_info = Box::leak(Box::new(AccountInfo::new(
            key_ref,
            false,
            false,
            lamports,
            data,
            owner_ref,
            false,
            Epoch::default(),
        )));

        UncheckedAccount::try_from(account_info)
    }

    #[test]
    fn check_is_initialized_rejects_noncanonical_buffer_state_key() {
        let accounts = BufferAccrualAccounts {
            buffer_state: unchecked_account_with_owner(Pubkey::new_unique(), crate::ID, 8),
            reserve_vault_onyc_account: unchecked_account_with_owner(
                Pubkey::new_unique(),
                crate::ID,
                0,
            ),
            management_fee_vault_onyc_account: unchecked_account_with_owner(
                Pubkey::new_unique(),
                crate::ID,
                0,
            ),
            performance_fee_vault_onyc_account: unchecked_account_with_owner(
                Pubkey::new_unique(),
                crate::ID,
                0,
            ),
        };

        let err = accounts.check_is_initialized(&crate::ID).unwrap_err();
        match err {
            Error::AnchorError(anchor_err) => assert_eq!(
                anchor_err.error_code_number,
                u32::from(BufferErrorCode::InvalidBufferStateAccount)
            ),
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn check_is_initialized_allows_uninitialized_canonical_buffer_state() {
        let (buffer_state_pda, _) = Pubkey::find_program_address(&[seeds::BUFFER_STATE], &crate::ID);
        let accounts = BufferAccrualAccounts {
            buffer_state: unchecked_account_with_owner(buffer_state_pda, Pubkey::new_unique(), 0),
            reserve_vault_onyc_account: unchecked_account_with_owner(
                Pubkey::new_unique(),
                crate::ID,
                0,
            ),
            management_fee_vault_onyc_account: unchecked_account_with_owner(
                Pubkey::new_unique(),
                crate::ID,
                0,
            ),
            performance_fee_vault_onyc_account: unchecked_account_with_owner(
                Pubkey::new_unique(),
                crate::ID,
                0,
            ),
        };

        assert!(!accounts.check_is_initialized(&crate::ID).unwrap());
    }
}
