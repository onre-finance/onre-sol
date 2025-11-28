use std::cmp::max;

use trident_fuzz::fuzzing::*;

use crate::types;
use crate::utility::clean_old_vectors;

use crate::types::OfferVector;
use crate::FuzzTest;

impl FuzzTest {
    // ##################################################################################################################
    // Invariants
    // ##################################################################################################################
    pub fn remove_admin_invariant(
        &mut self,
        state_after: types::State,
        state_before: types::State,
        removed_admin: Pubkey,
    ) {
        // Find first empty slot
        for i in 0..20 {
            if state_before.admins[i] == removed_admin {
                assert_eq!(
                    state_after.admins[i],
                    Pubkey::default(),
                    "Admin not removed"
                );
                return;
            }
        }
        panic!("Removed admin not found");
    }

    pub fn set_admin_invariant(
        &mut self,
        state_after: types::State,
        state_before: types::State,
        new_admin: Pubkey,
    ) {
        // Find first empty slot
        for i in 0..20 {
            if state_before.admins[i] == Pubkey::default() {
                assert_eq!(state_after.admins[i], new_admin, "Admin not set");
                return;
            }
        }
        panic!("No empty slot found");
    }

    pub fn propose_boss_invariant(
        &mut self,
        state_after: types::State,
        state_before: types::State,
        new_boss: Pubkey,
        old_boss: Pubkey,
    ) {
        assert_eq!(state_after.proposed_boss, new_boss, "Proposed boss not set");
        assert_eq!(state_before.boss, old_boss, "Boss not set");
    }

    pub fn accept_boss_invariant(
        &mut self,
        state_after: types::State,
        state_before: types::State,
        new_boss: Pubkey,
    ) {
        assert_eq!(
            state_before.proposed_boss, new_boss,
            "Proposed boss not set"
        );
        assert_eq!(
            state_after.proposed_boss,
            Pubkey::default(),
            "Proposed boss not cleared"
        );
        assert_eq!(state_after.boss, new_boss, "Boss not set");
    }

    #[allow(clippy::too_many_arguments)]
    pub fn take_offer_invariant(
        &mut self,
        offer_before: types::Offer,
        _offer_after: types::Offer,
        user_token_in_before: TokenAccountWithExtensions,
        user_token_in_after: TokenAccountWithExtensions,
        user_token_out_before: Option<TokenAccountWithExtensions>,
        user_token_out_after: TokenAccountWithExtensions,
        boss_token_in_before: TokenAccountWithExtensions,
        boss_token_in_after: TokenAccountWithExtensions,
        _vault_token_in_before: TokenAccountWithExtensions,
        _vault_token_in_after: TokenAccountWithExtensions,
        _vault_token_out_before: TokenAccountWithExtensions,
        _vault_token_out_after: TokenAccountWithExtensions,
        token_in_mint_before: MintWithExtensions,
        _token_in_mint_after: MintWithExtensions,
        token_out_mint_before: MintWithExtensions,
        _token_out_mint_after: MintWithExtensions,
        token_in_amount: u64,
    ) {
        // INVARIANT 1: User pays exactly token_in_amount
        let user_paid = user_token_in_before
            .account
            .amount
            .checked_sub(user_token_in_after.account.amount)
            .expect("User balance underflow");
        assert_eq!(
            user_paid, token_in_amount,
            "User must pay exactly token_in_amount"
        );

        // INVARIANT 2: Boss receives exactly token_in_amount
        let boss_received = boss_token_in_after
            .account
            .amount
            .checked_sub(boss_token_in_before.account.amount)
            .expect("Boss balance overflow");
        assert_eq!(
            boss_received, token_in_amount,
            "Boss must receive exactly token_in_amount"
        );

        // INVARIANT 3: User receives exact calculated token_out
        let user_token_out_before_amount =
            user_token_out_before.map(|a| a.account.amount).unwrap_or(0);
        let user_received = user_token_out_after
            .account
            .amount
            .checked_sub(user_token_out_before_amount)
            .expect("User token_out overflow");

        // Calculate what user should receive based on program logic
        let current_time = self.trident.get_current_timestamp() as u64;

        // Find active vector
        let active_vector = self
            .find_active_pricing_vector(&offer_before.vectors, current_time)
            .expect("No active vector found");

        // Calculate price
        let price = self.calculate_price(
            active_vector.apr,
            active_vector.base_price,
            active_vector.base_time,
            active_vector.price_fix_duration,
            current_time,
        );

        // Calculate fee and remaining
        let fee = (token_in_amount as u128 * offer_before.fee_basis_points as u128 / 10000) as u64;
        let remaining = token_in_amount - fee;

        // Calculate expected token_out
        let expected = self.calculate_token_out(
            remaining,
            price,
            token_in_mint_before.mint.decimals,
            token_out_mint_before.mint.decimals,
        );

        assert_eq!(
            user_received, expected,
            "User must receive exactly calculated token_out"
        );
    }

    // Helper function to find active vector at a specific time
    pub fn find_active_pricing_vector(
        &self,
        vectors: &[types::OfferVector],
        time: u64,
    ) -> Option<types::OfferVector> {
        vectors
            .iter()
            .filter(|v| v.start_time != 0 && v.start_time <= time)
            .max_by_key(|v| v.start_time)
            .cloned()
    }

    // Replicate offer_utils.rs::calculate_step_price_at
    pub fn calculate_price(
        &self,
        apr: u64,
        base_price: u64,
        base_time: u64,
        price_fix_duration: u64,
        current_time: u64,
    ) -> u64 {
        const SECONDS_IN_YEAR: u128 = 31_536_000;
        const APR_SCALE: u128 = 1_000_000;

        if base_time > current_time {
            return base_price;
        }

        let elapsed_since_start = current_time.saturating_sub(base_time);

        // Calculate which price interval we're in (discrete intervals)
        let current_step = elapsed_since_start / price_fix_duration;

        // elapsed_effective = (k + 1) * D  (end-of-current-interval snap)
        let step_end_time = (current_step + 1)
            .checked_mul(price_fix_duration)
            .expect("Step end time overflow");

        // Calculate price growth: P(t) = P0 * (1 + apr * elapsed_time / SECONDS_IN_YEAR)
        let factor_den = APR_SCALE
            .checked_mul(SECONDS_IN_YEAR)
            .expect("Factor denominator overflow");

        let y_part = (apr as u128)
            .checked_mul(step_end_time as u128)
            .expect("Y part overflow");

        let factor_num = factor_den
            .checked_add(y_part)
            .expect("Factor numerator overflow");

        let price_u128 = (base_price as u128)
            .checked_mul(factor_num)
            .expect("Price multiplication overflow")
            .checked_div(factor_den)
            .expect("Price division by zero");

        if price_u128 > u64::MAX as u128 {
            panic!("Price calculation overflow: {}", price_u128);
        }

        price_u128 as u64
    }

    // Replicate token_utils.rs::calculate_token_out_amount
    pub fn calculate_token_out(
        &self,
        token_in_amount: u64,
        price: u64,
        token_in_decimals: u8,
        token_out_decimals: u8,
    ) -> u64 {
        const PRICE_DECIMALS: u8 = 9;

        // token_out_amount = (token_in_amount * price * 10^token_out_decimals) / (10^PRICE_DECIMALS * 10^token_in_decimals)
        let numerator = (token_in_amount as u128)
            .checked_mul(10u128.pow(PRICE_DECIMALS as u32))
            .expect("Numerator multiplication overflow")
            .checked_mul(10u128.pow(token_out_decimals as u32))
            .expect("Numerator decimal scaling overflow");

        let denominator = (price as u128)
            .checked_mul(10u128.pow(token_in_decimals as u32))
            .expect("Denominator calculation overflow");

        let token_out = numerator
            .checked_div(denominator)
            .expect("Token out division by zero");

        if token_out > u64::MAX as u128 {
            panic!("Token out calculation overflow: {}", token_out);
        }

        token_out as u64
    }

    pub fn initialize_invariant(&mut self, state: Pubkey, boss: Pubkey, onyc_mint: Pubkey) {
        let state_account = self
            .trident
            .get_account_with_type::<types::State>(&state, 8)
            .expect("State account not found");

        assert_eq!(state_account.boss, boss, "Boss not initialized");
        assert_eq!(
            state_account.proposed_boss,
            Pubkey::default(),
            "Proposed boss not empty"
        );
        assert!(!state_account.is_killed, "State killed");
        assert_eq!(
            state_account.onyc_mint, onyc_mint,
            "ONyc mint not initialized"
        );
        assert_eq!(
            state_account.admins,
            [Pubkey::default(); 20],
            "Admins not empty"
        );
        assert_eq!(
            state_account.approver1,
            Pubkey::default(),
            "Approver1 not empty"
        );
        assert_eq!(
            state_account.approver2,
            Pubkey::default(),
            "Approver2 not empty"
        );
        assert_eq!(state_account.max_supply, 0, "Max supply not 0");
    }

    // fn initialize_vault_authority_invariant(&mut self, offer_vault_authority: Pubkey) {
    //     self.trident
    //         .get_account_with_type::<types::OfferVaultAuthority>(&offer_vault_authority, 8)
    //         .expect("State account not found");
    // }

    pub fn initialize_permissionless_authority_invariant(
        &mut self,
        permissionless_authority: Pubkey,
        name: String,
    ) {
        let permissionless_authority = self
            .trident
            .get_account_with_type::<types::PermissionlessAuthority>(&permissionless_authority, 8)
            .expect("Permissionless authority account not found");
        assert_eq!(
            permissionless_authority.name,
            name.trim(),
            "Permissionless authority name not set"
        );
    }

    // fn initialize_mint_authority_invariant(&mut self, offer_mint_authority: Pubkey) {
    //     self.trident
    //         .get_account_with_type::<types::MintAuthority>(&offer_mint_authority, 8)
    //         .expect("Mint authority account not found");
    // }

    pub fn offer_vault_deposit_invariant(
        &mut self,
        boss_before_deposit: TokenAccountWithExtensions,
        boss_after_deposit: TokenAccountWithExtensions,
        vault_token_account_before_deposit: Option<TokenAccountWithExtensions>,
        vault_token_account_after_deposit: TokenAccountWithExtensions,
        amount: u64,
    ) {
        assert_eq!(
            boss_before_deposit.account.amount - amount,
            boss_after_deposit.account.amount,
            "Boss balance not updated"
        );

        match vault_token_account_before_deposit {
            Some(vault_token_account_before_deposit) => {
                assert_eq!(
                    vault_token_account_before_deposit.account.amount + amount,
                    vault_token_account_after_deposit.account.amount,
                    "Vault token account balance not updated"
                );
            }
            None => {
                assert_eq!(
                    vault_token_account_after_deposit.account.amount, amount,
                    "Vault token account balance not updated"
                );
            }
        }
    }

    pub fn offer_vault_withdraw_invariant(
        &mut self,
        boss_before_withdraw: TokenAccountWithExtensions,
        boss_after_withdraw: TokenAccountWithExtensions,
        vault_token_account_before_withdraw: TokenAccountWithExtensions,
        vault_token_account_after_withdraw: TokenAccountWithExtensions,
        amount: u64,
    ) {
        assert_eq!(
            boss_before_withdraw.account.amount + amount,
            boss_after_withdraw.account.amount,
            "Boss balance not updated"
        );

        assert_eq!(
            vault_token_account_before_withdraw.account.amount - amount,
            vault_token_account_after_withdraw.account.amount,
            "Vault token account balance not updated"
        );
    }

    pub fn make_offer_invariant(
        &mut self,
        offer: Pubkey,
        fee_basis_points: u16,
        token_in_mint: Pubkey,
        token_out_mint: Pubkey,
        needs_approval: bool,
        allow_permissionless: bool,
    ) {
        let offer = self
            .trident
            .get_account_with_type::<types::Offer>(&offer, 8)
            .expect("Offer account not found");

        assert_eq!(offer.token_in_mint, token_in_mint, "Token in mint not set");
        assert_eq!(
            offer.token_out_mint, token_out_mint,
            "Token out mint not set"
        );

        offer.vectors.iter().all(|f| {
            f.start_time == 0
                && f.base_time == 0
                && f.base_price == 0
                && f.apr == 0
                && f.price_fix_duration == 0
        });

        assert_eq!(
            offer.fee_basis_points, fee_basis_points,
            "Fee basis points not set"
        );
        assert_eq!(
            offer.needs_approval, needs_approval as u8,
            "Needs approval not set"
        );
        assert_eq!(
            offer.allow_permissionless, allow_permissionless as u8,
            "Allow permissionless not set"
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_offer_vector_invariant(
        &mut self,
        offer_after: types::Offer,
        mut offer_before: types::Offer,
        start_time_opt: Option<u64>,
        base_time: u64,
        base_price: u64,
        apr: u64,
        price_fix_duration: u64,
        current_time: i64,
    ) {
        let start_time = start_time_opt.unwrap_or_else(|| max(current_time as u64, base_time));

        let new_vector = OfferVector {
            start_time,
            base_time,
            base_price,
            apr,
            price_fix_duration,
        };

        clean_old_vectors(&mut offer_before, &new_vector, current_time as u64);

        let empty_index = offer_before
            .vectors
            .iter()
            .position(|vector| vector.start_time == 0)
            .unwrap();

        assert_eq!(offer_after.vectors.len(), 10, "Offer vectors length not 10");
        assert_eq!(
            offer_after.vectors[empty_index].start_time, base_time,
            "Start time not set"
        );
        assert_eq!(
            offer_after.vectors[empty_index].base_time, base_time,
            "Base time not set"
        );
        assert_eq!(
            offer_after.vectors[empty_index].base_price, base_price,
            "Base price not set"
        );
        assert_eq!(offer_after.vectors[empty_index].apr, apr, "APR not set");
        assert_eq!(
            offer_after.vectors[empty_index].price_fix_duration, price_fix_duration,
            "Price fix duration not set"
        );
    }
}
