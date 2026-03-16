use anchor_lang::{prelude::*, system_program};

pub trait PdaAccountInit: AccountSerialize + AccountDeserialize {
    fn pda_seed_prefixes() -> &'static [&'static [u8]];
    fn init_space() -> usize;
    fn init_value(bump: u8) -> Self;
    fn invalid_owner_error() -> Error;
    fn invalid_data_error() -> Error;
}

pub fn load_or_init_pda_account<'info, T>(
    account: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    system_program_account: &AccountInfo<'info>,
    program_id: &Pubkey,
    bump: u8,
) -> Result<T>
where
    T: PdaAccountInit,
{
    let bump_seed = [bump];
    let mut signer_seed_parts: Vec<&[u8]> = T::pda_seed_prefixes().to_vec();
    signer_seed_parts.push(&bump_seed);
    let signer_seeds = [&signer_seed_parts[..]];

    if account.owner == &system_program::ID {
        if !account.data_is_empty() {
            return Err(T::invalid_owner_error());
        }

        let rent_lamports = Rent::get()?.minimum_balance(T::init_space());
        system_program::create_account(
            CpiContext::new_with_signer(
                system_program_account.clone(),
                system_program::CreateAccount {
                    from: payer.clone(),
                    to: account.clone(),
                },
                &signer_seeds,
            ),
            rent_lamports,
            T::init_space() as u64,
            program_id,
        )?;

        return Ok(T::init_value(bump));
    }

    if account.owner != program_id {
        return Err(T::invalid_owner_error());
    }

    let data = account.try_borrow_data()?;
    let mut slice: &[u8] = &data;
    T::try_deserialize(&mut slice).map_err(|_| T::invalid_data_error())
}
