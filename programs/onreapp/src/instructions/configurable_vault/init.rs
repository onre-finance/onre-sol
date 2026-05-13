use crate::constants::seeds;
use crate::state::{ConfigurableVault, ConfigurableVaultKind};
use crate::utils::PdaAccountInit;
use anchor_lang::prelude::*;

pub(crate) struct ConfigurableVaultInit<const KIND: u8> {
    bump: u8,
}

impl<const KIND: u8> ConfigurableVaultInit<KIND> {
    fn kind() -> ConfigurableVaultKind {
        match KIND {
            0 => ConfigurableVaultKind::OfferFee,
            1 => ConfigurableVaultKind::ManagementFee,
            2 => ConfigurableVaultKind::PerformanceFee,
            3 => ConfigurableVaultKind::PropAmmFee,
            _ => unreachable!("invalid configurable vault kind"),
        }
    }
}

impl<const KIND: u8> PdaAccountInit for ConfigurableVaultInit<KIND> {
    fn pda_seed_prefixes() -> &'static [&'static [u8]] {
        match Self::kind() {
            ConfigurableVaultKind::OfferFee => &[seeds::CONFIGURABLE_VAULT, seeds::OFFER_FEE_VAULT],
            ConfigurableVaultKind::ManagementFee => {
                &[seeds::CONFIGURABLE_VAULT, seeds::MANAGEMENT_FEE_VAULT]
            }
            ConfigurableVaultKind::PerformanceFee => {
                &[seeds::CONFIGURABLE_VAULT, seeds::PERFORMANCE_FEE_VAULT]
            }
            ConfigurableVaultKind::PropAmmFee => {
                &[seeds::CONFIGURABLE_VAULT, seeds::PROP_AMM_FEE_VAULT]
            }
        }
    }

    fn init_space() -> usize {
        8 + ConfigurableVault::INIT_SPACE
    }

    fn init_value(bump: u8) -> Self {
        Self { bump }
    }

    fn invalid_owner_error() -> Error {
        error!(crate::OnreError::InvalidConfigurableVaultOwner)
    }

    fn invalid_data_error() -> Error {
        error!(crate::OnreError::InvalidConfigurableVaultData)
    }
}

impl<const KIND: u8> AccountSerialize for ConfigurableVaultInit<KIND> {
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(ConfigurableVault::DISCRIMINATOR)?;
        AnchorSerialize::serialize(
            &ConfigurableVault {
                kind: Self::kind().as_u8(),
                withdrawal_destination: Pubkey::default(),
                bump: self.bump,
                reserved: [0; 31],
            },
            writer,
        )?;
        Ok(())
    }
}

impl<const KIND: u8> AccountDeserialize for ConfigurableVaultInit<KIND> {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
        let original = *buf;
        let vault = match ConfigurableVault::try_deserialize(buf) {
            Ok(vault) => vault,
            Err(_) => {
                let mut unchecked = original;
                let vault = ConfigurableVault::try_deserialize_unchecked(&mut unchecked)?;
                *buf = unchecked;
                vault
            }
        };
        require!(
            vault.kind == Self::kind().as_u8(),
            crate::OnreError::InvalidConfigurableVaultKind
        );
        Ok(Self { bump: vault.bump })
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        Self::try_deserialize(buf)
    }
}
