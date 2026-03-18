use super::*;

pub const MAX_VECTORS: usize = 10;

#[derive(Debug, Clone, Copy)]
pub struct OfferVectorData {
    pub start_time: u64,
    pub base_time: u64,
    pub base_price: u64,
    pub apr: u64,
    pub price_fix_duration: u64,
}

impl OfferVectorData {
    pub fn is_active(&self) -> bool {
        self.start_time != 0
    }
}

#[derive(Debug)]
pub struct OfferData {
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub vectors: [OfferVectorData; MAX_VECTORS],
    pub fee_basis_points: u16,
    pub bump: u8,
    pub needs_approval: u8,
    pub allow_permissionless: u8,
}

impl OfferData {
    pub fn active_vectors(&self) -> Vec<&OfferVectorData> {
        self.vectors.iter().filter(|v| v.is_active()).collect()
    }
}

pub fn read_offer(svm: &LiteSVM, token_in_mint: &Pubkey, token_out_mint: &Pubkey) -> OfferData {
    let (offer_pda, _) = find_offer_pda(token_in_mint, token_out_mint);
    let account = svm.get_account(&offer_pda).expect("offer account not found");
    let data = &account.data;
    let mut offset = 8;
    let tin = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;
    let tout = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;
    let mut vectors = [OfferVectorData {
        start_time: 0,
        base_time: 0,
        base_price: 0,
        apr: 0,
        price_fix_duration: 0,
    }; MAX_VECTORS];
    for vector in vectors.iter_mut() {
        let st = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let bt = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let bp = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let ap = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let pfd = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        *vector = OfferVectorData {
            start_time: st,
            base_time: bt,
            base_price: bp,
            apr: ap,
            price_fix_duration: pfd,
        };
    }
    let fee_basis_points = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
    offset += 2;
    let bump = data[offset];
    offset += 1;
    let needs_approval = data[offset];
    offset += 1;
    let allow_permissionless = data[offset];

    OfferData {
        token_in_mint: tin,
        token_out_mint: tout,
        vectors,
        fee_basis_points,
        bump,
        needs_approval,
        allow_permissionless,
    }
}

pub fn get_mint_supply(svm: &LiteSVM, mint: &Pubkey) -> u64 {
    let account = svm.get_account(mint).expect("mint account not found");
    u64::from_le_bytes(account.data[36..44].try_into().unwrap())
}

pub struct StateData {
    pub boss: Pubkey,
    pub proposed_boss: Pubkey,
    pub is_killed: bool,
    pub onyc_mint: Pubkey,
    pub admins: [Pubkey; MAX_ADMINS],
    pub approver1: Pubkey,
    pub approver2: Pubkey,
    pub bump: u8,
    pub max_supply: u64,
    pub redemption_admin: Pubkey,
    pub main_offer: Pubkey,
}

impl StateData {
    pub fn active_admins(&self) -> Vec<Pubkey> {
        let default_pubkey = Pubkey::default();
        self.admins
            .iter()
            .filter(|a| **a != default_pubkey)
            .copied()
            .collect()
    }
}

pub fn read_state(svm: &LiteSVM) -> StateData {
    let (state_pda, _) = find_state_pda();
    let account = svm.get_account(&state_pda).expect("state account not found");
    let mut data_slice = account.data.as_slice();
    let state =
        onreapp::state::State::try_deserialize(&mut data_slice).expect("failed to deserialize State account");
    StateData {
        boss: state.boss,
        proposed_boss: state.proposed_boss,
        is_killed: state.is_killed,
        onyc_mint: state.onyc_mint,
        admins: state.admins,
        approver1: state.approver1,
        approver2: state.approver2,
        bump: state.bump,
        max_supply: state.max_supply,
        redemption_admin: state.redemption_admin,
        main_offer: state.main_offer,
    }
}

pub struct BufferStateData {
    pub onyc_mint: Pubkey,
    pub buffer_admin: Pubkey,
    pub gross_yield: u64,
    pub lowest_supply: u64,
    pub management_fee_basis_points: u16,
    pub performance_fee_basis_points: u16,
    pub performance_fee_high_watermark: u64,
    pub last_accrual_timestamp: i64,
    pub bump: u8,
}

pub fn read_buffer_state(svm: &LiteSVM) -> BufferStateData {
    let (buffer_state_pda, _) = find_buffer_state_pda();
    let account = svm.get_account(&buffer_state_pda).expect("buffer state account not found");
    let mut data_slice = account.data.as_slice();
    let buffer_state = onreapp::instructions::BufferState::try_deserialize(&mut data_slice)
        .expect("failed to deserialize BufferState account");
    BufferStateData {
        onyc_mint: buffer_state.onyc_mint,
        buffer_admin: buffer_state.buffer_admin,
        gross_yield: buffer_state.gross_apr,
        lowest_supply: buffer_state.lowest_supply,
        management_fee_basis_points: buffer_state.management_fee_basis_points,
        performance_fee_basis_points: buffer_state.performance_fee_basis_points,
        performance_fee_high_watermark: buffer_state.performance_fee_high_watermark,
        last_accrual_timestamp: buffer_state.last_accrual_timestamp,
        bump: buffer_state.bump,
    }
}

pub fn get_mint_authority_pubkey(svm: &LiteSVM, mint: &Pubkey) -> Option<Pubkey> {
    let account = svm.get_account(mint)?;
    let tag = u32::from_le_bytes(account.data[0..4].try_into().unwrap());
    if tag == 1 {
        Some(Pubkey::try_from(&account.data[4..36]).unwrap())
    } else {
        None
    }
}

pub fn set_mint_authority(svm: &mut LiteSVM, mint: &Pubkey, new_authority: &Pubkey) {
    let mut account = svm.get_account(mint).expect("mint not found");
    account.data[0..4].copy_from_slice(&1u32.to_le_bytes());
    account.data[4..36].copy_from_slice(new_authority.as_ref());
    svm.set_account(*mint, account).unwrap();
}

pub fn get_return_u64(metadata: &litesvm::types::TransactionMetadata) -> u64 {
    u64::from_le_bytes(metadata.return_data.data[..8].try_into().unwrap())
}

pub fn get_return_i64(metadata: &litesvm::types::TransactionMetadata) -> i64 {
    i64::from_le_bytes(metadata.return_data.data[..8].try_into().unwrap())
}

pub struct RedemptionOfferData {
    pub offer: Pubkey,
    pub token_in_mint: Pubkey,
    pub token_out_mint: Pubkey,
    pub executed_redemptions: u128,
    pub requested_redemptions: u128,
    pub fee_basis_points: u16,
    pub request_counter: u64,
    pub bump: u8,
}

pub fn read_redemption_offer(
    svm: &LiteSVM,
    token_in_mint: &Pubkey,
    token_out_mint: &Pubkey,
) -> RedemptionOfferData {
    let (pda, _) = find_redemption_offer_pda(token_in_mint, token_out_mint);
    let account = svm.get_account(&pda).expect("redemption offer not found");
    let data = &account.data;
    let mut offset = 8;
    let offer = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;
    let tin = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;
    let tout = Pubkey::try_from(&data[offset..offset + 32]).unwrap();
    offset += 32;
    let executed_redemptions = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
    offset += 16;
    let requested_redemptions = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
    offset += 16;
    let fee_basis_points = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
    offset += 2;
    let request_counter = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    let bump = data[offset];
    RedemptionOfferData {
        offer,
        token_in_mint: tin,
        token_out_mint: tout,
        executed_redemptions,
        requested_redemptions,
        fee_basis_points,
        request_counter,
        bump,
    }
}

pub struct RedemptionRequestData {
    pub offer: Pubkey,
    pub request_id: u64,
    pub redeemer: Pubkey,
    pub amount: u64,
    pub fulfilled_amount: u64,
    pub bump: u8,
}

pub fn read_redemption_request(
    svm: &LiteSVM,
    redemption_offer: &Pubkey,
    request_id: u64,
) -> RedemptionRequestData {
    let (pda, _) = find_redemption_request_pda(redemption_offer, request_id);
    let account = svm
        .get_account(&pda)
        .expect("redemption request account not found");
    let mut data: &[u8] = &account.data;
    let request =
        RedemptionRequest::try_deserialize(&mut data).expect("Failed to deserialize RedemptionRequest");
    RedemptionRequestData {
        offer: request.offer,
        request_id: request.request_id,
        redeemer: request.redeemer,
        amount: request.amount,
        fulfilled_amount: request.fulfilled_amount,
        bump: request.bump,
    }
}
