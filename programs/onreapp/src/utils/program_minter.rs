//! Program-controlled minting of managed tokens.
//!
//! `ManagedTokenConfig::mint_multisig` selects how modular instructions mint:
//! - unset: the mint authority is the program's `["mint_authority"]` PDA, which signs directly;
//! - set: the mint authority is that 1-of-N SPL multisig, and the PDA signs as one of its
//!   members. Other members, such as a CCIP token pool, mint independently.
//!
//! The mode always comes from configuration, never from the mint's current authority,
//! so execution cannot silently fall back to another path. Legacy instructions keep
//! minting directly as the PDA only.
use crate::{constants::seeds, utils::token_utils::validate_mint_limits, OnreError};
use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke_signed, program_option::COption},
};
use anchor_spl::token_interface::{Mint, TokenInterface};
use solana_program::program_pack::Pack;
use spl_token_2022::{instruction::AuthorityType, state::Multisig};

/// Signing path for one managed mint, resolved and validated from its configuration.
pub(crate) struct ProgramMinter<'a, 'info> {
    mint_authority: &'a AccountInfo<'info>,
    multisig: Option<&'a AccountInfo<'info>>,
    bump: u8,
}

impl<'a, 'info> ProgramMinter<'a, 'info> {
    /// Requires the program to control `mint` in the configured mode. Direct mode
    /// requires the PDA as mint authority and no multisig account. Multisig mode
    /// requires the configured multisig as both the supplied account and the mint
    /// authority. Multisig contents are validated when the mode is enabled; SPL
    /// accounts cannot change their signer set afterwards.
    pub(crate) fn resolve(
        configured_multisig: Option<Pubkey>,
        mint: &InterfaceAccount<'info, Mint>,
        mint_authority: &'a AccountInfo<'info>,
        bump: u8,
        multisig: Option<&'a AccountInfo<'info>>,
    ) -> Result<Self> {
        let authority = match configured_multisig {
            None => {
                require!(multisig.is_none(), OnreError::UnexpectedMintMultisig);
                mint_authority.key()
            }
            Some(configured_multisig) => {
                let multisig = multisig.ok_or(OnreError::MintMultisigRequired)?;
                require_keys_eq!(
                    multisig.key(),
                    configured_multisig,
                    OnreError::InvalidMintMultisig
                );
                configured_multisig
            }
        };
        require!(
            mint.mint_authority == COption::Some(authority),
            OnreError::ProgramNotMintAuthority
        );
        Ok(Self {
            mint_authority,
            multisig,
            bump,
        })
    }

    /// Mints with the configured signing path after applying issuance limits.
    pub(crate) fn mint(
        &self,
        token_program: &Interface<'info, TokenInterface>,
        mint: &InterfaceAccount<'info, Mint>,
        to: &AccountInfo<'info>,
        amount: u64,
        max_supply: u64,
        max_mint_amount: u64,
    ) -> Result<()> {
        let Some(multisig) = self.multisig else {
            return crate::utils::mint_tokens(
                token_program,
                mint,
                to,
                self.mint_authority,
                &[&[seeds::MINT_AUTHORITY, &[self.bump]]],
                amount,
                max_supply,
                max_mint_amount,
            );
        };
        validate_mint_limits(mint, amount, max_supply, max_mint_amount)?;
        let instruction = spl_token_2022::instruction::mint_to_checked(
            &token_program.key(),
            &mint.key(),
            &to.key(),
            &multisig.key(),
            &[&self.mint_authority.key()],
            amount,
            mint.decimals,
        )?;
        invoke_signed(
            &instruction,
            &[
                mint.to_account_info(),
                to.clone(),
                multisig.clone(),
                self.mint_authority.clone(),
            ],
            &[&[seeds::MINT_AUTHORITY, &[self.bump]]],
        )
        .map_err(Into::into)
    }
}

/// Requires an initialized 1-of-N multisig of the mint's token program that has
/// the program's mint-authority PDA among its signers.
pub(crate) fn validate_mint_multisig(
    multisig: &AccountInfo,
    token_program: &Pubkey,
    mint_authority: &Pubkey,
) -> Result<()> {
    require_keys_eq!(
        *multisig.owner,
        *token_program,
        OnreError::InvalidMintMultisig
    );
    let data = multisig.try_borrow_data()?;
    require!(data.len() == Multisig::LEN, OnreError::InvalidMintMultisig);
    let state = Multisig::unpack(&data).map_err(|_| error!(OnreError::InvalidMintMultisig))?;
    require!(
        state.m == 1
            && state.signers[..state.n as usize]
                .iter()
                .any(|signer| signer == mint_authority),
        OnreError::InvalidMintMultisig
    );
    Ok(())
}

/// Moves mint authority from the multisig back to the program PDA, signing as
/// the PDA member of the 1-of-N multisig.
pub(crate) fn reclaim_mint_authority<'info>(
    token_program: &Interface<'info, TokenInterface>,
    mint: &InterfaceAccount<'info, Mint>,
    multisig: &AccountInfo<'info>,
    mint_authority: &AccountInfo<'info>,
    bump: u8,
) -> Result<()> {
    let instruction = spl_token_2022::instruction::set_authority(
        &token_program.key(),
        &mint.key(),
        Some(&mint_authority.key()),
        AuthorityType::MintTokens,
        &multisig.key(),
        &[&mint_authority.key()],
    )?;
    invoke_signed(
        &instruction,
        &[
            mint.to_account_info(),
            multisig.clone(),
            mint_authority.clone(),
        ],
        &[&[seeds::MINT_AUTHORITY, &[bump]]],
    )
    .map_err(Into::into)
}
