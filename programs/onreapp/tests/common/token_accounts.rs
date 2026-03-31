use super::*;

pub fn create_mint(
    svm: &mut LiteSVM,
    _payer: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
) -> Pubkey {
    let mint = Keypair::new();
    let mut mint_data = vec![0u8; 82];
    mint_data[0..4].copy_from_slice(&1u32.to_le_bytes());
    mint_data[4..36].copy_from_slice(mint_authority.as_ref());
    mint_data[44] = decimals;
    mint_data[45] = 1;
    mint_data[46..50].copy_from_slice(&1u32.to_le_bytes());
    mint_data[50..82].copy_from_slice(mint_authority.as_ref());

    svm.set_account(
        mint.pubkey(),
        Account {
            executable: false,
            data: mint_data,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_PROGRAM_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    mint.pubkey()
}

pub fn create_token_account(
    svm: &mut LiteSVM,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> Pubkey {
    let ata = get_associated_token_address(owner, mint);
    let mut token_data = vec![0u8; 165];
    token_data[0..32].copy_from_slice(mint.as_ref());
    token_data[32..64].copy_from_slice(owner.as_ref());
    token_data[64..72].copy_from_slice(&amount.to_le_bytes());
    token_data[108] = 1;

    svm.set_account(
        ata,
        Account {
            executable: false,
            data: token_data,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_PROGRAM_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    if amount > 0 {
        let mut mint_account = svm.get_account(mint).expect("mint account not found");
        let current_supply = u64::from_le_bytes(mint_account.data[36..44].try_into().unwrap());
        let new_supply = current_supply
            .checked_add(amount)
            .expect("mint supply overflow while seeding token account");
        mint_account.data[36..44].copy_from_slice(&new_supply.to_le_bytes());
        svm.set_account(*mint, mint_account).unwrap();
    }

    ata
}

pub fn create_mint_2022(
    svm: &mut LiteSVM,
    _payer: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
) -> Pubkey {
    let mint = Keypair::new();
    let mut mint_data = vec![0u8; 82];
    mint_data[0..4].copy_from_slice(&1u32.to_le_bytes());
    mint_data[4..36].copy_from_slice(mint_authority.as_ref());
    mint_data[44] = decimals;
    mint_data[45] = 1;
    mint_data[46..50].copy_from_slice(&1u32.to_le_bytes());
    mint_data[50..82].copy_from_slice(mint_authority.as_ref());

    svm.set_account(
        mint.pubkey(),
        Account {
            executable: false,
            data: mint_data,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_2022_PROGRAM_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    mint.pubkey()
}

pub fn create_token_account_2022(
    svm: &mut LiteSVM,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> Pubkey {
    let ata = get_associated_token_address_2022(owner, mint);
    let mut token_data = vec![0u8; 165];
    token_data[0..32].copy_from_slice(mint.as_ref());
    token_data[32..64].copy_from_slice(owner.as_ref());
    token_data[64..72].copy_from_slice(&amount.to_le_bytes());
    token_data[108] = 1;

    svm.set_account(
        ata,
        Account {
            executable: false,
            data: token_data,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_2022_PROGRAM_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    if amount > 0 {
        let mut mint_account = svm.get_account(mint).expect("mint account not found");
        let current_supply = u64::from_le_bytes(mint_account.data[36..44].try_into().unwrap());
        let new_supply = current_supply
            .checked_add(amount)
            .expect("mint supply overflow while seeding token account");
        mint_account.data[36..44].copy_from_slice(&new_supply.to_le_bytes());
        svm.set_account(*mint, mint_account).unwrap();
    }

    ata
}

pub fn create_mint_2022_with_transfer_fee(
    svm: &mut LiteSVM,
    _payer: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
    fee_basis_points: u16,
    max_fee: u64,
) -> Pubkey {
    let mint = Keypair::new();
    let mut mint_data = vec![0u8; 278];
    mint_data[0..4].copy_from_slice(&1u32.to_le_bytes());
    mint_data[4..36].copy_from_slice(mint_authority.as_ref());
    mint_data[44] = decimals;
    mint_data[45] = 1;
    mint_data[46..50].copy_from_slice(&1u32.to_le_bytes());
    mint_data[50..82].copy_from_slice(mint_authority.as_ref());
    mint_data[165] = 1;
    mint_data[166..168].copy_from_slice(&1u16.to_le_bytes());
    mint_data[168..170].copy_from_slice(&108u16.to_le_bytes());
    mint_data[170..202].copy_from_slice(mint_authority.as_ref());
    mint_data[202..234].copy_from_slice(mint_authority.as_ref());
    mint_data[250..258].copy_from_slice(&max_fee.to_le_bytes());
    mint_data[258..260].copy_from_slice(&fee_basis_points.to_le_bytes());
    mint_data[268..276].copy_from_slice(&max_fee.to_le_bytes());
    mint_data[276..278].copy_from_slice(&fee_basis_points.to_le_bytes());

    svm.set_account(
        mint.pubkey(),
        Account {
            executable: false,
            data: mint_data,
            lamports: INITIAL_LAMPORTS,
            owner: TOKEN_2022_PROGRAM_ID,
            rent_epoch: 0,
        },
    )
    .unwrap();

    mint.pubkey()
}
