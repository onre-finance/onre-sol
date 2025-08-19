use anchor_lang::prelude::*;

/// Represents an offer in the Onre App program.
///
/// Stores details of an offer where a boss provides buy tokens in exchange for sell tokens.
/// Used in `make_offer`, `take_offer`, and `close_offer` instructions.
///
/// # Fields
/// - `offer_id`: Unique identifier for the offer.
/// - `sell_token_start_amount`: Initial amount of sell tokens required at offer start.
/// - `sell_token_end_amount`: Final amount of sell tokens required at offer end.
/// - `sell_token_mint`: Mint of the token the offer expects to receive.
/// - `buy_token_1`: First buy token details (mint and amount).
/// - `buy_token_2`: Second buy token details (mint and amount, defaults if unused).
/// - `authority_bump`: Bump seed for the offer's token authority PDA.
/// - `price_fix_duration`: Duration in seconds for each fixed price interval.
/// - `offer_start_time`: Unix timestamp when the offer becomes active.
/// - `offer_end_time`: Unix timestamp when the offer expires.
#[account]
#[derive(InitSpace)]
pub struct Offer {
    pub offer_id: u64,
    pub sell_token_start_amount: u64,
    pub sell_token_end_amount: u64,
    pub sell_token_mint: Pubkey,
    pub buy_token_1: OfferToken,
    pub buy_token_2: OfferToken,
    pub authority_bump: u8,
    pub price_fix_duration: u64,
    pub offer_start_time: u64,
    pub offer_end_time: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct OfferToken {
    pub mint: Pubkey,
    pub amount: u64,
}

/// Represents the program state in the Onre App program.
///
/// Stores the current boss's public key, used for authorization across instructions.
///
/// # Fields
/// - `boss`: Public key of the current boss, set via `initialize` and updated via `set_boss`.
#[account]
#[derive(InitSpace)]
pub struct State {
    pub boss: Pubkey,
}

#[account]
#[derive(InitSpace)]
pub struct PermissionlessAccount {
    #[max_len(50)]
    pub name: String,
}
