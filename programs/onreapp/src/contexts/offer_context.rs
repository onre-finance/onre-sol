use crate::state::Offer;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;

pub trait CloseOfferContext<'info> {
  fn token_program(&self) -> &Program<'info, Token>;
  fn offer_token_authority(&self) -> &AccountInfo<'info>;
  fn offer(&self) -> &Account<'info, Offer>;
}

pub trait MakeOfferContext<'info> {
  fn token_program(&self) -> &Program<'info, Token>;
  fn boss(&self) -> &AccountInfo<'info>;
  fn offer(&self) -> &Account<'info, Offer>;
}

