use crate::{
    constants::seeds,
    model::{
        seeds::{BUFFER_MANAGEMENT, BUFFER_PERFORMANCE, BUFFER_RESERVE, VAULT},
        VaultConfig, VaultKind,
    },
    state::{ConfigurableVault, ConfigurableVaultKind},
    OnreError,
};
use anchor_lang::prelude::*;

/// Validated custody identity. Selecting a legacy vault does not change its role
/// or withdrawal policy; the boss explicitly authorizes its use by an offer.
pub(crate) enum VaultAuthority {
    Modular {
        kind: [u8; 1],
        instance_id: [u8; 8],
        bump: [u8; 1],
    },
    Legacy {
        kind: ConfigurableVaultKind,
        bump: [u8; 1],
    },
    Redemption {
        bump: [u8; 1],
    },
    Buffer {
        role: &'static [u8],
        mint: Pubkey,
        bump: [u8; 1],
    },
}

impl VaultAuthority {
    /// Seeds are returned only after validating the exact custody identity.
    pub fn signer_seeds(&self) -> Vec<&[u8]> {
        match self {
            Self::Modular {
                kind,
                instance_id,
                bump,
            } => vec![VAULT, kind, instance_id, bump],
            Self::Legacy { kind, bump } => vec![seeds::CONFIGURABLE_VAULT, kind.seed(), bump],
            Self::Redemption { bump } => vec![seeds::REDEMPTION_OFFER_VAULT_AUTHORITY, bump],
            Self::Buffer { role, mint, bump } => vec![role, mint.as_ref(), bump],
        }
    }
}

/// Validate a typed VaultConfig, including the token-specific BUFFER vaults.
/// BUFFER custody operations use the managed mint encoded in their PDA seeds.
pub(crate) fn validate_vault_config(
    key: &Pubkey,
    vault: &VaultConfig,
    mint: &Pubkey,
    program_id: &Pubkey,
) -> Result<VaultAuthority> {
    let kind = [vault.kind as u8];
    let instance_id = vault.instance_id.to_le_bytes();
    let (expected, bump) = Pubkey::find_program_address(&[VAULT, &kind, &instance_id], program_id);
    if *key == expected {
        require!(vault.bump == bump, OnreError::InvalidConfigurableVault);
        return Ok(VaultAuthority::Modular {
            kind,
            instance_id,
            bump: [bump],
        });
    }
    let role = match (vault.kind, vault.instance_id) {
        (VaultKind::BufferReserve, 0) => BUFFER_RESERVE,
        (VaultKind::Fee, 1) => BUFFER_MANAGEMENT,
        (VaultKind::Fee, 2) => BUFFER_PERFORMANCE,
        _ => return err!(OnreError::InvalidConfigurableVault),
    };
    let (expected, bump) = Pubkey::find_program_address(&[role, mint.as_ref()], program_id);
    require_keys_eq!(*key, expected, OnreError::InvalidConfigurableVault);
    require!(vault.bump == bump, OnreError::InvalidConfigurableVault);
    Ok(VaultAuthority::Buffer {
        role,
        mint: *mint,
        bump: [bump],
    })
}

/// Accept modular Liquidity, legacy configurable, or canonical redemption custody.
/// Configurations require owner, discriminator, kind, PDA and bump checks;
/// the stateless redemption authority requires its canonical PDA identity.
pub(crate) fn validate_liquidity_vault(
    account: &AccountInfo,
    program_id: &Pubkey,
) -> Result<VaultAuthority> {
    let (redemption, bump) =
        Pubkey::find_program_address(&[seeds::REDEMPTION_OFFER_VAULT_AUTHORITY], program_id);
    if account.key() == redemption {
        // This is a signing identity, not a serialized configuration account.
        return Ok(VaultAuthority::Redemption { bump: [bump] });
    }
    require_keys_eq!(
        *account.owner,
        *program_id,
        OnreError::InvalidConfigurableVaultOwner
    );
    let data = account.try_borrow_data()?;
    if data.starts_with(VaultConfig::DISCRIMINATOR) {
        let vault = VaultConfig::try_deserialize(&mut data.as_ref())
            .map_err(|_| error!(OnreError::InvalidConfigurableVaultData))?;
        require!(
            vault.kind == VaultKind::Liquidity,
            OnreError::InvalidConfigurableVaultKind
        );
        validate_vault_config(account.key, &vault, &Pubkey::default(), program_id)
    } else {
        let vault = ConfigurableVault::try_deserialize(&mut data.as_ref())
            .map_err(|_| error!(OnreError::InvalidConfigurableVaultData))?;
        let kind = match vault.kind {
            0 => ConfigurableVaultKind::OfferFee,
            1 => ConfigurableVaultKind::ManagementFee,
            2 => ConfigurableVaultKind::PerformanceFee,
            3 => ConfigurableVaultKind::PropAmmBuyFee,
            4 => ConfigurableVaultKind::OfferProceeds,
            5 => ConfigurableVaultKind::PropAmmProceeds,
            6 => ConfigurableVaultKind::PermissionlessOfferFee,
            7 => ConfigurableVaultKind::RedemptionFee,
            8 => ConfigurableVaultKind::PropAmmSellFee,
            _ => return err!(OnreError::InvalidConfigurableVaultKind),
        };
        let (expected, bump) =
            Pubkey::find_program_address(&[seeds::CONFIGURABLE_VAULT, kind.seed()], program_id);
        require!(vault.bump == bump, OnreError::InvalidConfigurableVault);
        require_keys_eq!(account.key(), expected, OnreError::InvalidConfigurableVault);
        Ok(VaultAuthority::Legacy { kind, bump: [bump] })
    }
}
