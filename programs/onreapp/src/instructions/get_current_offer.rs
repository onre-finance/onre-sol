use anchor_lang::prelude::*;


#[derive(Accounts)]
pub struct GetCurrentOffer {

}

pub const OFFER_ID: u64 = 1;

pub fn get_current_offer(_ctx: Context<GetCurrentOffer>) -> Result<u64> {
    Ok(OFFER_ID)
}

