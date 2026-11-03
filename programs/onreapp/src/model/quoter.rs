use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuoterKind {
    Nav,
    NavPermissionless,
    PropAmm,
}

/// Reusable quote dispatch. Prop AMM configuration lives in a companion PropAmmState.
#[account]
#[derive(InitSpace)]
pub struct Quoter {
    pub kind: QuoterKind,
    pub instance_id: u64,
    pub enabled: bool,
    pub bump: u8,
}
