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
