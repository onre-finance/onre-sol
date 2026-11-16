use anchor_lang::prelude::*;

/// Escrowed input of a Worker-execution offer, filled by the worker at the price
/// and fee current at each fill. PDA `["fulfillment_request", offer_config, user,
/// request_id_le]`; the escrow is the request's canonical ATA of the input mint.
/// Closed, with its escrow, once fully filled or cancelled; rent returns to `user`.
#[account]
#[derive(InitSpace)]
pub struct FulfillmentRequest {
    pub offer_config: Pubkey,
    pub user: Pubkey,
    /// User-chosen identifier, unique per offer and user among open requests.
    pub request_id: u64,
    /// Gross input escrowed at creation, in input mint base units.
    pub input_amount: u64,
    /// Gross input already filled; the remainder stays escrowed.
    pub fulfilled_input_amount: u64,
    pub bump: u8,
}

impl FulfillmentRequest {
    /// Runs `f` with the request PDA's signer seeds, which sign for its escrow.
    pub fn with_signer_seeds<R>(&self, f: impl FnOnce(&[&[u8]]) -> R) -> R {
        let request_id = self.request_id.to_le_bytes();
        f(&[
            super::seeds::FULFILLMENT_REQUEST,
            self.offer_config.as_ref(),
            self.user.as_ref(),
            &request_id,
            &[self.bump],
        ])
    }

    pub fn remaining_input_amount(&self) -> u64 {
        self.input_amount - self.fulfilled_input_amount
    }
}
