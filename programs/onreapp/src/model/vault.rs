use anchor_lang::prelude::*;

/// Reusable business purpose; instance identity is independent of its consumers.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultKind {
    Fee,
    Proceeds,
    Liquidity,
    BufferReserve,
    /// Managed tokens retained from Prop AMM sell haircuts instead of being burned.
    Haircut,
}

/// EVM ConfigurableVault equivalent; distinct name preserves the legacy discriminator.
///
/// The target SPL custody representation is one ATA per (vault PDA, mint).
/// Existing physical vaults are not moved or rebound by this model definition.
#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    pub kind: VaultKind,
    pub instance_id: u64,
    /// Fixed destination owner; withdrawals pay its ATA for the selected mint.
    /// Fee/Proceeds allow any caller; Liquidity/BufferReserve require State::boss.
    pub withdrawal_destination: Pubkey,
    /// Liquidity target as a fraction of managed-token TVL; zero disables refill.
    pub refill_target_bps: u16,
    pub bump: u8,
}

impl VaultConfig {
    /// Whether `key` is this vault's generic `["vault", kind, instance_id]` address,
    /// as opposed to a token-specific BUFFER vault, which can only hold its own mint.
    pub fn is_generic(&self, key: &Pubkey, program_id: &Pubkey) -> bool {
        Pubkey::create_program_address(
            &[
                super::seeds::VAULT,
                &[self.kind as u8],
                &self.instance_id.to_le_bytes(),
                &[self.bump],
            ],
            program_id,
        )
        .is_ok_and(|address| address == *key)
    }
}
