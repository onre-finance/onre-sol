use crate::{
    constants::seeds,
    model::{OfferDirection, OfferFlow},
    utils::{burn_tokens, mint_tokens, transfer_tokens},
    OnreError, ID,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;

use super::{liquidity, quote::OfferQuote, validation::ExecutionPlan, TakeOffer};

/// Transfer input fees, then settle the configured direction atomically.
pub(super) fn settle(
    a: &TakeOffer,
    plan: &ExecutionPlan,
    quote: &OfferQuote,
    authority_bump: u8,
    max_supply: u64,
    max_mint: u64,
) -> Result<()> {
    // Keep the same guard at settlement so a future dispatch change cannot
    // accidentally execute another quoter through NAV settlement.
    plan.require_supported()?;
    match plan.flow {
        OfferFlow::Permissioned => settle_collected_input(
            a,
            SettlementRoute {
                input: &a.user_token_in,
                output: &a.user_token_out,
                authority: a.user.to_account_info(),
                signer_seeds: None,
            },
            plan,
            quote,
            authority_bump,
            max_supply,
            max_mint,
        ),
        OfferFlow::Permissionless => {
            let authority = a
                .permissionless_authority
                .as_ref()
                .ok_or(OnreError::WrongAuthority)?;
            let input = a
                .permissionless_token_in
                .as_ref()
                .ok_or(OnreError::InvalidVaultTokenInAccount)?;
            let output = a
                .permissionless_token_out
                .as_ref()
                .ok_or(OnreError::InvalidPermissionlessTokenOutAccount)?;
            let gross = quote
                .net_input_amount
                .checked_add(quote.fee_amount)
                .ok_or(OnreError::MathOverflow)?;
            transfer_tokens(
                &a.token_in_mint,
                &a.token_in_program,
                &a.user_token_in,
                input,
                &a.user.to_account_info(),
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
                    authority: authority.to_account_info(),
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
                &a.token_out_mint,
                &a.token_out_program,
                output,
                &a.user_token_out,
                &authority.to_account_info(),
                Some(signer_seeds),
                quote.amount_out,
            )
        }
        OfferFlow::Worker => err!(OnreError::UnsupportedOfferExecution),
    }
}

struct SettlementRoute<'a, 'info> {
    input: &'a InterfaceAccount<'info, TokenAccount>,
    output: &'a InterfaceAccount<'info, TokenAccount>,
    authority: AccountInfo<'info>,
    signer_seeds: Option<&'a [&'a [&'a [u8]]]>,
}

fn settle_collected_input<'info>(
    a: &TakeOffer<'info>,
    route: SettlementRoute<'_, 'info>,
    plan: &ExecutionPlan,
    quote: &OfferQuote,
    authority_bump: u8,
    max_supply: u64,
    max_mint: u64,
) -> Result<()> {
    if quote.fee_amount > 0 {
        transfer_tokens(
            &a.token_in_mint,
            &a.token_in_program,
            route.input,
            &a.fee_token_in,
            &route.authority,
            route.signer_seeds,
            quote.fee_amount,
        )?;
    }
    match plan.direction {
        OfferDirection::AssetToManaged => {
            let proceeds = a
                .proceeds_token_in
                .as_ref()
                .ok_or(OnreError::InvalidConfigurableVaultTokenAccount)?;
            transfer_tokens(
                &a.token_in_mint,
                &a.token_in_program,
                route.input,
                proceeds,
                &route.authority,
                route.signer_seeds,
                quote.net_input_amount,
            )?;
            mint_tokens(
                &a.token_out_program,
                &a.token_out_mint,
                &route.output.to_account_info(),
                &a.mint_authority,
                &[&[seeds::MINT_AUTHORITY, &[authority_bump]]],
                quote.amount_out,
                max_supply,
                max_mint,
            )?;
        }
        OfferDirection::ManagedToAsset => {
            burn_tokens(
                &a.token_in_program,
                &a.token_in_mint,
                route.input,
                &route.authority,
                route.signer_seeds.unwrap_or(&[]),
                quote.net_input_amount,
            )?;
            liquidity::consume(a, &plan.liquidity, quote.amount_out, route.output)?;
        }
    }
    Ok(())
}
