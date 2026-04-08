use crate::constants::seeds;
use crate::utils::transfer_tokens;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FeeKind {
    Management,
    Performance,
}

impl FeeKind {
    fn vault_authority_seed(self) -> &'static [u8] {
        match self {
            Self::Management => seeds::MANAGEMENT_FEE_VAULT_AUTHORITY,
            Self::Performance => seeds::PERFORMANCE_FEE_VAULT_AUTHORITY,
        }
    }
}

pub(crate) fn withdraw_fee_tokens<'info>(
    fee_kind: FeeKind,
    onyc_mint: &InterfaceAccount<'info, Mint>,
    token_program: &Interface<'info, TokenInterface>,
    fee_vault_onyc_account: &InterfaceAccount<'info, TokenAccount>,
    recipient_onyc_account: &InterfaceAccount<'info, TokenAccount>,
    fee_vault_authority: &AccountInfo<'info>,
    authority_bump: u8,
    amount: u64,
) -> Result<()> {
    require!(
        amount <= fee_vault_onyc_account.amount,
        crate::OnreError::InsufficientFeeBalance
    );

    let authority_bump_seed = [authority_bump];
    let authority_seeds = &[fee_kind.vault_authority_seed(), &authority_bump_seed[..]];
    let signer_seeds = &[&authority_seeds[..]];

    transfer_tokens(
        onyc_mint,
        token_program,
        fee_vault_onyc_account,
        recipient_onyc_account,
        fee_vault_authority,
        Some(signer_seeds),
        amount,
    )
}
