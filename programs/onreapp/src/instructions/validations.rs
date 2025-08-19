fn validate_non_zero_token_amounts(token_amounts: &[u64]) -> Result<()> {
    require!(
        token_amounts.iter().all(|&x| x > 0),
        MakeOfferErrorCode::InvalidAmount
    );
    Ok(())
}