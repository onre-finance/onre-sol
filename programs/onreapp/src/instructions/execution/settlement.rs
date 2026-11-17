use crate::{
    constants::seeds,
    model::{OfferConfig, OfferDirection},
    utils::{burn_tokens, ceil_div_u128, transfer_tokens},
    OnreError, ID,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;

use super::{
    liquidity, quote::OfferQuote, validation, validation::ExecutionPlan, Execution, Route,
};

/// Transfer input fees, then settle the configured direction atomically.
pub(super) fn settle(
    a: &Execution,
    plan: &ExecutionPlan,
    quote: &OfferQuote,
    authority_bump: u8,
    max_supply: u64,
    max_mint: u64,
) -> Result<()> {
    // Keep the same guard at settlement so a future dispatch change cannot
    // accidentally execute another quoter through NAV settlement.
    validation::require_supported(a)?;
    match a.route {
        Route::Direct {
            input,
            output,
            user,
        } => settle_collected_input(
            a,
            SettlementRoute {
                input,
                output,
                authority: user.clone(),
                signer_seeds: None,
            },
            plan,
            quote,
            authority_bump,
            max_supply,
            max_mint,
        ),
        Route::Intermediary {
            user_input,
            user_output,
            user,
            authority,
            input,
            output,
        } => {
            let gross = quote
                .net_input_amount
                .checked_add(quote.fee_amount)
                .ok_or(OnreError::MathOverflow)?;
            transfer_tokens(
                a.token_in_mint,
                a.token_in_program,
                user_input,
                input,
                user,
                None,
                gross,
            )?;
            let (_, bump) = Pubkey::find_program_address(&[seeds::PERMISSIONLESS_AUTHORITY], &ID);
            let authority_seeds: &[&[u8]] = &[seeds::PERMISSIONLESS_AUTHORITY, &[bump]];
            let signer_seeds = &[authority_seeds];
            settle_collected_input(
                a,
                SettlementRoute {
                    input,
                    output,
                    authority: authority.clone(),
                    signer_seeds: Some(signer_seeds),
                },
                plan,
                quote,
                authority_bump,
                max_supply,
                max_mint,
            )?;
            // Transfer exactly this execution's output. Existing intermediary
            // balances are never swept into the user's payout.
            transfer_tokens(
                a.token_out_mint,
                a.token_out_program,
                output,
                user_output,
                authority,
                Some(signer_seeds),
                quote.amount_out,
            )
        }
        Route::Escrow {
            request,
            input,
            output,
        } => request.with_signer_seeds(|request_seeds| {
            settle_collected_input(
                a,
                SettlementRoute {
                    input,
                    output,
                    authority: request.to_account_info(),
                    signer_seeds: Some(&[request_seeds]),
                },
                plan,
                quote,
                authority_bump,
                max_supply,
                max_mint,
            )
        }),
    }
}

struct SettlementRoute<'a, 'info> {
    input: &'a InterfaceAccount<'info, TokenAccount>,
    output: &'a InterfaceAccount<'info, TokenAccount>,
    authority: AccountInfo<'info>,
    signer_seeds: Option<&'a [&'a [&'a [u8]]]>,
}

fn settle_collected_input<'info>(
    a: &Execution<'_, 'info>,
    route: SettlementRoute<'_, 'info>,
    plan: &ExecutionPlan,
    quote: &OfferQuote,
    authority_bump: u8,
    max_supply: u64,
    max_mint: u64,
) -> Result<()> {
    if quote.fee_amount > 0 {
        transfer_tokens(
            a.token_in_mint,
            a.token_in_program,
            route.input,
            a.fee_token_in,
            &route.authority,
            route.signer_seeds,
            quote.fee_amount,
        )?;
    }
    match plan.direction {
        OfferDirection::AssetToManaged => {
            let proceeds = a
                .proceeds_token_in
                .ok_or(OnreError::InvalidConfigurableVaultTokenAccount)?;
            liquidity::deposit_destination(
                a,
                route.input,
                &route.authority,
                route.signer_seeds,
                quote.liquidity_refill_amount,
            )?;
            transfer_tokens(
                a.token_in_mint,
                a.token_in_program,
                route.input,
                proceeds,
                &route.authority,
                route.signer_seeds,
                quote.net_input_amount - quote.liquidity_refill_amount,
            )?;
            if a.config.offer_config.source_vault().is_some() {
                liquidity::consume(a, quote.amount_out, route.output)?;
            } else {
                a.minter(authority_bump)?.mint(
                    a.token_out_program,
                    a.token_out_mint,
                    &route.output.to_account_info(),
                    quote.amount_out,
                    max_supply,
                    max_mint,
                )?;
            }
        }
        OfferDirection::ManagedToAsset => {
            if a.config.offer_config.destination_vault().is_some() {
                liquidity::deposit_destination(
                    a,
                    route.input,
                    &route.authority,
                    route.signer_seeds,
                    quote.net_input_amount,
                )?;
            } else {
                // Burn only the share the seller was paid for; a Prop AMM haircut
                // share is retained in the offer's Haircut vault.
                let retained = retained_haircut(&a.config.offer_config, quote)?;
                burn_tokens(
                    a.token_in_program,
                    a.token_in_mint,
                    route.input,
                    &route.authority,
                    route.signer_seeds.unwrap_or(&[]),
                    quote.net_input_amount - retained,
                )?;
                liquidity::deposit_haircut(
                    a,
                    route.input,
                    &route.authority,
                    route.signer_seeds,
                    retained,
                )?;
            }
            liquidity::consume(a, quote.amount_out, route.output)?;
        }
    }
    Ok(())
}

/// Managed input retained by a sell's Prop AMM haircut: the net input minus the
/// share paid for, `ceil(net × paid / raw)`, so the program never burns less than
/// it paid out. Zero without a haircut vault, with a destination (which receives the
/// whole net input), or when nothing was withheld.
pub(super) fn retained_haircut(offer: &OfferConfig, quote: &OfferQuote) -> Result<u64> {
    if offer.direction != OfferDirection::ManagedToAsset
        || offer.destination_vault().is_some()
        || offer.haircut_vault().is_none()
        || quote.amount_out >= quote.raw_amount_out
    {
        return Ok(0);
    }
    let paid = ceil_div_u128(
        (quote.net_input_amount as u128)
            .checked_mul(quote.amount_out as u128)
            .ok_or(OnreError::MathOverflow)?,
        quote.raw_amount_out as u128,
    )
    .ok_or(OnreError::MathOverflow)?;
    Ok(quote.net_input_amount - paid as u64)
}
