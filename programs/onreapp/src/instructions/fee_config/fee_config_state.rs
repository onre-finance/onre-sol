use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;

/// Discriminator for fee configuration types.
///
/// Each variant maps to a separate FeeConfig PDA, allowing independent
/// fee routing per operation type.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FeeType {
    TakeOffer = 0,
    FulfillRedemption = 1,
}

/// Per-operation-type fee routing configuration.
///
/// Each FeeConfig PDA controls where fees are sent for a specific operation type.
/// When `destination` is `None`, fees accumulate in the PDA's own ATA and can be
/// withdrawn by the boss via `withdraw_fees`. When `destination` is `Some(addr)`,
/// fees are sent directly to `addr`'s ATA during the operation.
///
/// PDA seeds: `[b"fee_config", &[fee_type as u8]]`
#[account]
#[derive(InitSpace)]
pub struct FeeConfig {
    /// The FeeType discriminator this config applies to
    pub fee_type: u8,
    /// Optional override destination; None means fees go to this PDA's own ATA
    pub destination: Option<Pubkey>,
    /// PDA bump seed
    pub bump: u8,
    /// Reserved space for future extensions
    pub reserved: [u8; 64],
}

impl FeeConfig {
    /// Returns the wallet that should own the fee ATA: the configured
    /// `destination` address, or the FeeConfig PDA itself when unset.
    pub fn fee_destination_owner(&self, fee_config_key: &Pubkey) -> Pubkey {
        self.destination.unwrap_or(*fee_config_key)
    }

    /// Derives the expected associated token account for fee collection.
    pub fn fee_destination_ata(
        &self,
        fee_config_key: &Pubkey,
        token_mint: &Pubkey,
        token_program: &Pubkey,
    ) -> Pubkey {
        get_associated_token_address_with_program_id(
            &self.fee_destination_owner(fee_config_key),
            token_mint,
            token_program,
        )
    }

    /// Validates that `actual_fee_ata` matches the expected fee destination ATA
    /// derived from the current configuration.
    pub fn validate_fee_destination(
        &self,
        fee_config_key: &Pubkey,
        actual_fee_ata: &Pubkey,
        token_mint: &Pubkey,
        token_program: &Pubkey,
    ) -> Result<()> {
        let expected_ata = self.fee_destination_ata(fee_config_key, token_mint, token_program);
        require_keys_eq!(
            *actual_fee_ata,
            expected_ata,
            FeeConfigError::InvalidFeeDestination
        );
        Ok(())
    }
}

#[error_code]
pub enum FeeConfigError {
    #[msg("Invalid fee destination account")]
    InvalidFeeDestination,
}