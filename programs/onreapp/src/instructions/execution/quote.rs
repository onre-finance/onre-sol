use crate::{
    constants::{MAX_BASIS_POINTS, PRICE_DECIMALS},
    model::{FeeConfig, OfferDirection, Pricer},
    utils::{
        calculate_fees, calculate_token_out_amount, pricing::get_active_vector_and_current_price,
        MAX_TOKEN_DECIMALS,
    },
    OnreError,
};
use anchor_lang::prelude::*;

pub struct OfferQuote {
    pub price: u64,
    pub fee_amount: u64,
    pub net_input_amount: u64,
    pub amount_out: u64,
}

pub fn quote_offer(
    pricer: &Pricer,
    fee: &FeeConfig,
    gross_input: u64,
    direction: OfferDirection,
    input_decimals: u8,
    output_decimals: u8,
    now: u64,
) -> Result<OfferQuote> {
    require!(gross_input > 0, OnreError::InvalidAmount);
    require!(fee.basis_points <= MAX_BASIS_POINTS, OnreError::InvalidFee);
    let fee_amount = calculate_fees(gross_input, fee.basis_points)?
        .token_in_fee_amount
        .max(fee.minimum_fee_amount);
    require!(fee_amount < gross_input, OnreError::InvalidAmount);
    let net_input_amount = gross_input - fee_amount;
    let (_, price) = get_active_vector_and_current_price(pricer.vectors.iter().copied(), now)?;
    Ok(OfferQuote {
        price,
        fee_amount,
        net_input_amount,
        amount_out: match direction {
            OfferDirection::AssetToManaged => calculate_token_out_amount(
                net_input_amount,
                price,
                input_decimals,
                output_decimals,
            )?,
            OfferDirection::ManagedToAsset => {
                managed_to_asset_amount(net_input_amount, price, input_decimals, output_decimals)?
            }
        },
    })
}

/// Direct NAV multiplication, with the same rounding as legacy redemption pricing.
fn managed_to_asset_amount(
    amount: u64,
    price: u64,
    input_decimals: u8,
    output_decimals: u8,
) -> Result<u64> {
    require!(
        input_decimals <= MAX_TOKEN_DECIMALS && output_decimals <= MAX_TOKEN_DECIMALS,
        OnreError::DecimalsExceedMax
    );
    let numerator = (amount as u128)
        .checked_mul(price as u128)
        .and_then(|value| value.checked_mul(10_u128.pow(output_decimals as u32)))
        .ok_or(OnreError::MathOverflow)?;
    let denominator = 10_u128.pow((input_decimals + PRICE_DECIMALS) as u32);
    let result = numerator / denominator;
    require!(result > 0, OnreError::InvalidAmount);
    u64::try_from(result).map_err(|_| OnreError::MathOverflow.into())
}
