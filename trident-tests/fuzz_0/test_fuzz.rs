use fuzz_accounts::*;
use trident_fuzz::fuzzing::*;
mod fuzz_accounts;
mod types;
use types::*;
mod invariants;
mod utility;

use crate::types::onreapp::AcceptBossInstruction;
use crate::types::onreapp::AcceptBossInstructionAccounts;
use crate::types::onreapp::AcceptBossInstructionData;
use crate::types::onreapp::AddOfferVectorInstructionData;
use crate::types::onreapp::InitializeInstructionAccounts;
use crate::types::onreapp::InitializeInstructionData;
use crate::types::onreapp::MakeOfferInstructionAccounts;
use crate::types::onreapp::MakeOfferInstructionData;
use crate::types::onreapp::ProposeBossInstruction;
use crate::types::onreapp::ProposeBossInstructionAccounts;
use crate::types::onreapp::ProposeBossInstructionData;

pub const TOKEN_2022_PROGRAM_ID: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const BOSS: Pubkey = pubkey!("DSnL9GnR7kbAXHwCvcXBuDTfFvezcszUGGyiMz9Nug4o");

#[derive(Default)]
pub struct TokenPairs {
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub token_program_in: Pubkey,
    pub token_program_out: Pubkey,
}

#[derive(FuzzTestMethods)]
struct FuzzTest {
    /// Trident client for interacting with the Solana program
    trident: Trident,
    /// Storage for all account addresses used in fuzz testing
    fuzz_accounts: AccountAddresses,

    token_mixing: TokenPairs,
}

#[flow_executor]
impl FuzzTest {
    fn new() -> Self {
        Self {
            trident: Trident::default(),
            fuzz_accounts: AccountAddresses::default(),
            token_mixing: TokenPairs::default(),
        }
    }

    #[init]
    fn start(&mut self) {
        self.fuzz_accounts.boss.insert_with_address(BOSS);

        let boss = self
            .fuzz_accounts
            .boss
            .get(&mut self.trident)
            .expect("Boss Storage empty");

        self.trident.airdrop(&boss, 500 * LAMPORTS_PER_SOL);

        let payer = self.fuzz_accounts.payer.insert(&mut self.trident, None);

        self.trident.airdrop(&payer, 100 * LAMPORTS_PER_SOL);

        let random_token_authority = self
            .fuzz_accounts
            .random_token_authority
            .insert(&mut self.trident, None);

        self.setup_onyc_mint_and_initialize(payer, boss, random_token_authority);

        let random_name = self.trident.random_string(10);
        self.setup_permissionless_authority(random_name);

        let token_mint_a = self
            .fuzz_accounts
            .token_mint_a
            .insert(&mut self.trident, None);

        self.setup_mint(payer, token_mint_a, 4, random_token_authority);

        let token_mint_2022_a = self
            .fuzz_accounts
            .token_mint_2022_a
            .insert(&mut self.trident, None);

        self.setup_mint_2022(payer, token_mint_2022_a, 4, random_token_authority);

        let token_mint_b = self
            .fuzz_accounts
            .token_mint_b
            .insert(&mut self.trident, None);

        self.setup_mint(payer, token_mint_b, 9, random_token_authority);

        let token_mint_2022_b = self
            .fuzz_accounts
            .token_mint_2022_b
            .insert(&mut self.trident, None);

        self.setup_mint_2022(payer, token_mint_2022_b, 4, random_token_authority);

        self.setup_associated_token_account(payer, token_mint_a, boss);

        self.setup_associated_token_account_2022(payer, token_mint_2022_a, boss);

        self.setup_associated_token_account(payer, token_mint_b, boss);

        self.setup_associated_token_account_2022(payer, token_mint_2022_b, boss);

        self.mint_to_ata(
            token_mint_a,
            boss,
            100_000_000_000_000,
            random_token_authority,
        );
        self.mint_to_2022_ata(
            token_mint_2022_a,
            boss,
            100_000_000_000_000,
            random_token_authority,
        );
        self.mint_to_ata(
            token_mint_b,
            boss,
            100_000_000_000_000,
            random_token_authority,
        );
        self.mint_to_2022_ata(
            token_mint_2022_b,
            boss,
            100_000_000_000_000,
            random_token_authority,
        );

        let deposit_amount = self.trident.random_from_range(1000..100_000_000_000_000);

        self.deposit_to_vault(
            deposit_amount,
            token_mint_a,
            boss,
            Some("deposit_to_vault_a"),
            TOKEN_PROGRAM_ID,
        );

        self.deposit_to_vault(
            deposit_amount,
            token_mint_2022_a,
            boss,
            Some("deposit_to_vault_2022_a"),
            TOKEN_2022_PROGRAM_ID,
        );

        self.deposit_to_vault(
            deposit_amount,
            token_mint_b,
            boss,
            Some("deposit_to_vault_b"),
            TOKEN_PROGRAM_ID,
        );

        self.deposit_to_vault(
            deposit_amount,
            token_mint_2022_b,
            boss,
            Some("deposit_to_vault_2022_b"),
            TOKEN_2022_PROGRAM_ID,
        );

        let withdraw_amount = self.trident.random_from_range(1000..2000);

        self.withdraw_from_vault(
            withdraw_amount,
            token_mint_a,
            boss,
            Some("withdraw_from_vault_a"),
            TOKEN_PROGRAM_ID,
        );

        self.withdraw_from_vault(
            withdraw_amount,
            token_mint_2022_a,
            boss,
            Some("withdraw_from_vault_2022_a"),
            TOKEN_2022_PROGRAM_ID,
        );

        self.withdraw_from_vault(
            withdraw_amount,
            token_mint_b,
            boss,
            Some("withdraw_from_vault_b"),
            TOKEN_PROGRAM_ID,
        );

        self.withdraw_from_vault(
            withdraw_amount,
            token_mint_2022_b,
            boss,
            Some("withdraw_from_vault_2022_b"),
            TOKEN_2022_PROGRAM_ID,
        );

        let vault_authority = self
            .fuzz_accounts
            .offer_vault_authority
            .get(&mut self.trident)
            .expect("Offer Vault Authority Storage empty");

        self.mint_to_ata(
            token_mint_a,
            vault_authority,
            100_000_000_000_000,
            random_token_authority,
        );

        self.mint_to_2022_ata(
            token_mint_2022_a,
            vault_authority,
            100_000_000_000_000,
            random_token_authority,
        );
        self.mint_to_ata(
            token_mint_b,
            vault_authority,
            100_000_000_000_000,
            random_token_authority,
        );

        self.mint_to_2022_ata(
            token_mint_2022_b,
            vault_authority,
            100_000_000_000_000,
            random_token_authority,
        );

        let user = self.fuzz_accounts.user.insert(&mut self.trident, None);

        self.setup_associated_token_account(payer, token_mint_a, user);

        self.setup_associated_token_account_2022(payer, token_mint_2022_a, user);

        self.setup_associated_token_account(payer, token_mint_b, user);

        self.setup_associated_token_account_2022(payer, token_mint_2022_b, user);

        self.mint_to_ata(
            token_mint_a,
            user,
            100_000_000_000_000,
            random_token_authority,
        );

        self.mint_to_2022_ata(
            token_mint_2022_a,
            user,
            100_000_000_000_000,
            random_token_authority,
        );

        let fee_basis_points = self.trident.random_from_range(1..1_000);
        let needs_approval = false;
        let allow_permissionless = false;

        self.token_mixing.token_in = token_mint_2022_a;
        self.token_mixing.token_out = token_mint_b;
        self.token_mixing.token_program_in = TOKEN_2022_PROGRAM_ID;
        self.token_mixing.token_program_out = TOKEN_PROGRAM_ID;

        self.make_offer(
            fee_basis_points,
            needs_approval,
            allow_permissionless,
            Some("Making offer, token_mint_2022_a = token_in, token_mint_b = token_out"),
        );
    }

    #[flow]
    fn flow1(&mut self) {
        let base_time = self.trident.get_current_timestamp() as u64;
        let base_price = self.trident.random_from_range(100..100_000_000_000);
        let apr = self.trident.random_from_range(0..100_000_000_000);
        let price_fix_duration = self.trident.random_from_range(100..100_000);

        let start_time = if self.trident.random_bool() {
            None
        } else {
            Some(base_time)
        };

        self.add_offer_vector(
            start_time,
            base_time,
            base_price,
            apr,
            price_fix_duration,
            Some("Adding offer vector, token_mint_a = token_in, token_mint_b = token_out"),
        );

        self.trident.forward_in_time(100_000_000);

        let user = self
            .fuzz_accounts
            .user
            .get(&mut self.trident)
            .expect("User Storage empty");

        self.take_offer(
            user,
            100,
            Some("Taking offer, token_mint_a = token_in, token_mint_b = token_out"),
        );
    }

    #[flow]
    fn flow2(&mut self) {
        let x = self.trident.random_from_range(0..2);
        let old_boss = if x == 0 {
            self.fuzz_accounts
                .boss
                .get(&mut self.trident)
                .expect("Boss Storage empty")
        } else {
            self.trident.random_pubkey()
        };
        let new_boss = self.trident.random_pubkey();

        // set boss to new boss
        self.propose_boss(old_boss, new_boss);

        // set boss back to old boss
        self.accept_boss(new_boss);

        // set boss to new boss
        self.propose_boss(new_boss, old_boss);

        // set boss back to old boss
        self.accept_boss(old_boss);
    }

    #[flow]
    fn flow3(&mut self) {
        let new_admin = self.fuzz_accounts.admins.insert(&mut self.trident, None);

        // set admin to new admin
        self.set_admin(new_admin);
    }

    #[flow]
    fn flow4(&mut self) {
        let x = self.trident.random_from_range(0..2);
        let removed_admin = if x == 0 {
            if self.fuzz_accounts.admins.is_empty() {
                self.trident.random_pubkey()
            } else {
                self.fuzz_accounts
                    .admins
                    .get(&mut self.trident)
                    .expect("Admins Storage empty")
            }
        } else {
            self.trident.random_pubkey()
        };

        // set admin to new admin
        self.remove_admin(removed_admin);
    }

    #[end]
    fn end(&mut self) {
        // Perform any cleanup here, this method will be process_transactiond
        // at the end of each iteration
    }

    // ##################################################################################################################
    // Helper functions
    // ##################################################################################################################

    fn remove_admin(&mut self, removed_admin: Pubkey) {
        let state = self
            .fuzz_accounts
            .state
            .get(&mut self.trident)
            .expect("State Storage empty");
        let boss = self
            .fuzz_accounts
            .boss
            .get(&mut self.trident)
            .expect("Boss Storage empty");

        let state_before = self
            .trident
            .get_account_with_type::<types::State>(&state, 8)
            .expect("State account not found");

        let remove_admin = types::onreapp::RemoveAdminInstruction::data(
            onreapp::RemoveAdminInstructionData::new(removed_admin),
        )
        .accounts(onreapp::RemoveAdminInstructionAccounts::new(state, boss))
        .instruction();

        let res = self
            .trident
            .process_transaction(&[remove_admin], Some("remove_admin"));

        if res.is_success() {
            let state_after = self
                .trident
                .get_account_with_type::<types::State>(&state, 8)
                .expect("State account not found");
            self.remove_admin_invariant(state_after, state_before, removed_admin);
        } else {
            let state_before = self
                .trident
                .get_account_with_type::<types::State>(&state, 8)
                .expect("State account not found");

            assert_eq!(
                !state_before.admins.contains(&removed_admin),
                res.is_custom_error_with_code(6000_u32),
                "Failed to remove admin: removed_admin: {:#?} is in admins, but got error: {:#?}",
                removed_admin,
                res.logs()
            );
        }
    }
    fn set_admin(&mut self, new_admin: Pubkey) {
        let state = self
            .fuzz_accounts
            .state
            .get(&mut self.trident)
            .expect("State Storage empty");
        let boss = self
            .fuzz_accounts
            .boss
            .get(&mut self.trident)
            .expect("Boss Storage empty");

        let state_before = self
            .trident
            .get_account_with_type::<types::State>(&state, 8)
            .expect("State account not found");

        let set_admin =
            onreapp::AddAdminInstruction::data(onreapp::AddAdminInstructionData::new(new_admin))
                .accounts(onreapp::AddAdminInstructionAccounts::new(state, boss))
                .instruction();

        let res = self
            .trident
            .process_transaction(&[set_admin], Some("set_admin"));

        if res.is_success() {
            let state_after = self
                .trident
                .get_account_with_type::<types::State>(&state, 8)
                .expect("State account not found");

            self.set_admin_invariant(state_after, state_before, new_admin);
        } else {
            assert!(
                !res.is_success(),
                "Failed to set admin: logs: {:#?}",
                res.logs()
            );
        }
    }

    fn propose_boss(&mut self, old_boss: Pubkey, new_boss: Pubkey) {
        let state = self
            .fuzz_accounts
            .state
            .get(&mut self.trident)
            .expect("State Storage empty");

        let state_before = self
            .trident
            .get_account_with_type::<types::State>(&state, 8)
            .expect("State account not found");

        let propose_boss = ProposeBossInstruction::data(ProposeBossInstructionData::new(new_boss))
            .accounts(ProposeBossInstructionAccounts::new(state, old_boss))
            .instruction();

        let res = self
            .trident
            .process_transaction(&[propose_boss], Some("propose_boss"));

        let state_after = self
            .trident
            .get_account_with_type::<types::State>(&state, 8)
            .expect("State account not found");

        if res.is_success() {
            self.propose_boss_invariant(state_after, state_before, new_boss, old_boss);
        } else {
            assert_eq!(
                state_before.boss != old_boss,
                res.is_custom_error_with_code(2001_u32),
                "Failed to set boss, with logs: {:#?}",
                res.logs()
            );
        }
    }

    fn accept_boss(&mut self, new_boss: Pubkey) {
        let state = self
            .fuzz_accounts
            .state
            .get(&mut self.trident)
            .expect("State Storage empty");

        let state_before = self
            .trident
            .get_account_with_type::<types::State>(&state, 8)
            .expect("State account not found");

        let accept_boss = AcceptBossInstruction::data(AcceptBossInstructionData::new())
            .accounts(AcceptBossInstructionAccounts::new(state, new_boss))
            .instruction();

        let res = self
            .trident
            .process_transaction(&[accept_boss], Some("accept_boss"));

        let state_after = self
            .trident
            .get_account_with_type::<types::State>(&state, 8)
            .expect("State account not found");

        if res.is_success() {
            self.accept_boss_invariant(state_after, state_before, new_boss);
        } else {
            assert_eq!(
                state_before.proposed_boss != new_boss,
                res.is_custom_error_with_code(6000_u32),
                "Failed to accept boss, with logs: {:#?}",
                res.logs()
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn take_offer(&mut self, user: Pubkey, token_in_amount: u64, message: Option<&str>) {
        let offer = self
            .fuzz_accounts
            .offer
            .get(&mut self.trident)
            .expect("Offer Storage empty");
        let state = self
            .fuzz_accounts
            .state
            .get(&mut self.trident)
            .expect("State Storage empty");
        let boss = self
            .fuzz_accounts
            .boss
            .get(&mut self.trident)
            .expect("Boss Storage empty");
        let vault_authority = self
            .fuzz_accounts
            .offer_vault_authority
            .get(&mut self.trident)
            .expect("Offer Vault Authority Storage empty");

        let mint_authority = self
            .fuzz_accounts
            .offer_mint_authority
            .get(&mut self.trident)
            .expect("Offer Mint Authority Storage empty");

        let user_token_in_account = self.trident.get_associated_token_address(
            &self.token_mixing.token_in,
            &user,
            &self.token_mixing.token_program_in,
        );
        let user_token_out_account = self.trident.get_associated_token_address(
            &self.token_mixing.token_out,
            &user,
            &self.token_mixing.token_program_out,
        );

        let boss_token_in_account = self.trident.get_associated_token_address(
            &self.token_mixing.token_in,
            &boss,
            &self.token_mixing.token_program_in,
        );

        let vault_token_in_account = self.trident.get_associated_token_address(
            &self.token_mixing.token_in,
            &vault_authority,
            &self.token_mixing.token_program_in,
        );
        let vault_token_out_account = self.trident.get_associated_token_address(
            &self.token_mixing.token_out,
            &vault_authority,
            &self.token_mixing.token_program_out,
        );

        // Capture balances before transaction
        let offer_before = self
            .trident
            .get_account_with_type::<types::Offer>(&offer, 8)
            .expect("Offer account not found");

        let user_token_in_before = self
            .trident
            .get_token_account(user_token_in_account)
            .expect("User token_in account not found");

        let user_token_out_before = self.trident.get_token_account(user_token_out_account).ok();

        let boss_token_in_before = self
            .trident
            .get_token_account(boss_token_in_account)
            .expect("Boss token_in account not found");

        let vault_token_in_before = self
            .trident
            .get_token_account(vault_token_in_account)
            .expect("Vault token_in account not found");

        let vault_token_out_before = self
            .trident
            .get_token_account(vault_token_out_account)
            .expect("Vault token_out account not found");

        let token_in_mint_before = self
            .trident
            .get_mint(self.token_mixing.token_in)
            .expect("Token_in mint not found");

        let token_out_mint_before = self
            .trident
            .get_mint(self.token_mixing.token_out)
            .expect("Token_out mint not found");

        let take_offer = onreapp::TakeOfferInstruction::data(
            onreapp::TakeOfferInstructionData::new(token_in_amount, None),
        )
        .accounts(onreapp::TakeOfferInstructionAccounts {
            offer,
            state,
            boss,
            vault_authority,
            vault_token_in_account,
            vault_token_out_account,
            token_in_mint: self.token_mixing.token_in,
            token_in_program: self.token_mixing.token_program_in,
            token_out_mint: self.token_mixing.token_out,
            token_out_program: self.token_mixing.token_program_out,
            user_token_in_account,
            user_token_out_account,
            boss_token_in_account,
            mint_authority,
            user,
        })
        .instruction();

        let res = self.trident.process_transaction(&[take_offer], message);

        if !res.is_success() {
            panic!("Failed to take offer: logs: {:#?}", res.logs());
        } else {
            // Capture balances after transaction
            let offer_after = self
                .trident
                .get_account_with_type::<types::Offer>(&offer, 8)
                .expect("Offer account not found");

            let user_token_in_after = self
                .trident
                .get_token_account(user_token_in_account)
                .expect("User token_in account not found");

            let user_token_out_after = self
                .trident
                .get_token_account(user_token_out_account)
                .expect("User token_out account not found");

            let boss_token_in_after = self
                .trident
                .get_token_account(boss_token_in_account)
                .expect("Boss token_in account not found");

            let vault_token_in_after = self
                .trident
                .get_token_account(vault_token_in_account)
                .expect("Vault token_in account not found");

            let vault_token_out_after = self
                .trident
                .get_token_account(vault_token_out_account)
                .expect("Vault token_out account not found");

            let token_in_mint_after = self
                .trident
                .get_mint(self.token_mixing.token_in)
                .expect("Token_in mint not found");

            let token_out_mint_after = self
                .trident
                .get_mint(self.token_mixing.token_out)
                .expect("Token_out mint not found");

            self.take_offer_invariant(
                offer_before,
                offer_after,
                user_token_in_before,
                user_token_in_after,
                user_token_out_before,
                user_token_out_after,
                boss_token_in_before,
                boss_token_in_after,
                vault_token_in_before,
                vault_token_in_after,
                vault_token_out_before,
                vault_token_out_after,
                token_in_mint_before,
                token_in_mint_after,
                token_out_mint_before,
                token_out_mint_after,
                token_in_amount,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add_offer_vector(
        &mut self,
        start_time: Option<u64>,
        base_time: u64,
        base_price: u64,
        apr: u64,
        price_fix_duration: u64,
        message: Option<&str>,
    ) {
        let state = self
            .fuzz_accounts
            .state
            .get(&mut self.trident)
            .expect("State Storage empty");
        let boss = self
            .fuzz_accounts
            .boss
            .get(&mut self.trident)
            .expect("Boss Storage empty");
        let offer = self
            .fuzz_accounts
            .offer
            .get(&mut self.trident)
            .expect("Offer Storage empty");

        let offer_before = self
            .trident
            .get_account_with_type::<types::Offer>(&offer, 8)
            .expect("Offer account not found");

        let add_offer_vector =
            onreapp::AddOfferVectorInstruction::data(AddOfferVectorInstructionData::new(
                start_time,
                base_time,
                base_price,
                apr,
                price_fix_duration,
            ))
            .accounts(onreapp::AddOfferVectorInstructionAccounts::new(
                offer,
                self.token_mixing.token_in,
                self.token_mixing.token_out,
                state,
                boss,
            ))
            .instruction();

        let current_time = self.trident.get_current_timestamp();
        let res = self
            .trident
            .process_transaction(&[add_offer_vector], message);

        if res.is_success() {
            let offer_after = self
                .trident
                .get_account_with_type::<types::Offer>(&offer, 8)
                .expect("Offer account not found");
            self.add_offer_vector_invariant(
                offer_after,
                offer_before,
                start_time,
                base_time,
                base_price,
                apr,
                price_fix_duration,
                current_time,
            );
        } else {
            let has_empty_slot = offer_before
                .vectors
                .iter()
                .any(|vector| vector.start_time == 0);

            assert_eq!(
                !has_empty_slot,
                res.is_custom_error_with_code(6003_u32),
                "Expected TooManyVectors error when offer is full"
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn make_offer(
        &mut self,
        fee_basis_points: u16,
        needs_approval: bool,
        allow_permissionless: bool,
        message: Option<&str>,
    ) {
        let boss = self
            .fuzz_accounts
            .boss
            .get(&mut self.trident)
            .expect("Boss Storage empty");
        let state = self
            .fuzz_accounts
            .state
            .get(&mut self.trident)
            .expect("State Storage empty");

        let vault_authority = self
            .fuzz_accounts
            .offer_vault_authority
            .get(&mut self.trident)
            .expect("Offer Vault Authority Storage empty");

        let vault_token_account_a = self.trident.get_associated_token_address(
            &self.token_mixing.token_in,
            &vault_authority,
            &self.token_mixing.token_program_in,
        );

        let offer = self.fuzz_accounts.offer.insert(
            &mut self.trident,
            Some(PdaSeeds::new(
                &[
                    b"offer",
                    self.token_mixing.token_in.as_ref(),
                    self.token_mixing.token_out.as_ref(),
                ],
                types::onreapp::program_id(),
            )),
        );

        let make_offer = onreapp::MakeOfferInstruction::data(MakeOfferInstructionData::new(
            fee_basis_points,
            needs_approval,
            allow_permissionless,
        ))
        .accounts(MakeOfferInstructionAccounts::new(
            vault_authority,
            self.token_mixing.token_in,
            self.token_mixing.token_program_in,
            vault_token_account_a,
            self.token_mixing.token_out,
            offer,
            state,
            boss,
        ))
        .instruction();

        let res = self.trident.process_transaction(&[make_offer], message);

        assert!(
            res.is_success(),
            "Failed to make offer: logs: {:#?}",
            res.logs()
        );

        self.make_offer_invariant(
            offer,
            fee_basis_points,
            self.token_mixing.token_in,
            self.token_mixing.token_out,
            needs_approval,
            allow_permissionless,
        );
    }

    fn mint_to_2022_ata(
        &mut self,
        token_mint: Pubkey,
        owner: Pubkey,
        amount: u64,
        mint_authority: Pubkey,
    ) {
        let token_account =
            self.trident
                .get_associated_token_address(&token_mint, &owner, &TOKEN_2022_PROGRAM_ID);
        let ixs = self
            .trident
            .mint_to_2022(&token_account, &token_mint, &mint_authority, amount);

        let res = self.trident.process_transaction(&[ixs], None);

        assert!(
            res.is_success(),
            "Failed to mint to token account: logs: {:#?}",
            res.logs()
        );
    }

    fn mint_to_ata(
        &mut self,
        token_mint: Pubkey,
        owner: Pubkey,
        amount: u64,
        mint_authority: Pubkey,
    ) {
        let token_account =
            self.trident
                .get_associated_token_address(&token_mint, &owner, &TOKEN_PROGRAM_ID);
        let ix = self
            .trident
            .mint_to(&token_account, &token_mint, &mint_authority, amount);

        let res = self.trident.process_transaction(&[ix], None);

        assert!(
            res.is_success(),
            "Failed to mint to token account: logs: {:#?}",
            res.logs()
        );
    }

    fn setup_associated_token_account_2022(
        &mut self,
        payer: Pubkey,
        token_mint: Pubkey,
        owner: Pubkey,
    ) {
        let ixs =
            self.trident
                .initialize_associated_token_account_2022(&payer, &token_mint, &owner, &[]);

        let res = self.trident.process_transaction(&ixs, None);

        assert!(
            res.is_success(),
            "Failed to initialize associated token account: logs: {:#?}",
            res.logs()
        );
    }

    fn setup_associated_token_account(&mut self, payer: Pubkey, token_mint: Pubkey, owner: Pubkey) {
        let ix = self
            .trident
            .initialize_associated_token_account(&payer, &token_mint, &owner);

        let res = self.trident.process_transaction(&[ix], None);
        assert!(
            res.is_success(),
            "Failed to initialize associated token account: logs: {:#?}",
            res.logs()
        );
    }

    fn setup_mint_2022(
        &mut self,
        payer: Pubkey,
        address: Pubkey,
        decimals: u8,
        mint_authority: Pubkey,
    ) {
        let ixs = self.trident.initialize_mint_2022(
            &payer,
            &address,
            decimals,
            &mint_authority,
            None,
            &[],
        );

        let res = self.trident.process_transaction(&ixs, None);

        assert!(
            res.is_success(),
            "Failed to initialize mint: logs: {:#?}",
            res.logs()
        );
    }

    fn setup_mint(&mut self, payer: Pubkey, address: Pubkey, decimals: u8, mint_authority: Pubkey) {
        let ixs = self
            .trident
            .initialize_mint(&payer, &address, decimals, &mint_authority, None);

        let res = self.trident.process_transaction(&ixs, None);

        assert!(
            res.is_success(),
            "Failed to initialize mint: logs: {:#?}",
            res.logs()
        );
    }

    fn deposit_to_vault(
        &mut self,
        deposit_amount: u64,
        token_mint: Pubkey,
        from: Pubkey,
        message: Option<&str>,
        token_program_id: Pubkey,
    ) {
        let offer_vault_authority = self
            .fuzz_accounts
            .offer_vault_authority
            .get(&mut self.trident)
            .expect("Offer Vault Authority Storage empty");

        let boss_ata =
            self.trident
                .get_associated_token_address(&token_mint, &from, &token_program_id);

        let vault_token_account = self.trident.get_associated_token_address(
            &token_mint,
            &offer_vault_authority,
            &token_program_id,
        );

        let boss_before_deposit = self
            .trident
            .get_token_account(boss_ata)
            .expect("Failed to get boss token account before deposit");

        let offer_vault_authority = self
            .fuzz_accounts
            .offer_vault_authority
            .get(&mut self.trident)
            .expect("Offer Vault Authority Storage empty");

        let state = self
            .fuzz_accounts
            .state
            .get(&mut self.trident)
            .expect("State Storage empty");

        let boss = self
            .fuzz_accounts
            .boss
            .get(&mut self.trident)
            .expect("Boss Storage empty");

        let vault_token_account_before_deposit =
            self.trident.get_token_account(vault_token_account).ok();

        let vault_deposit = types::onreapp::OfferVaultDepositInstruction::data(
            onreapp::OfferVaultDepositInstructionData::new(deposit_amount),
        )
        .accounts(onreapp::OfferVaultDepositInstructionAccounts::new(
            offer_vault_authority,
            token_mint,
            boss_ata,
            vault_token_account,
            boss,
            state,
            token_program_id,
        ))
        .instruction();

        let res = self.trident.process_transaction(&[vault_deposit], message);

        assert!(
            res.is_success(),
            "Failed to deposit to vault: logs: {:#?}",
            res.logs()
        );

        let vault_token_account_after_deposit = self
            .trident
            .get_token_account(vault_token_account)
            .expect("Failed to get vault token account after deposit");

        let boss_after_deposit = self
            .trident
            .get_token_account(boss_ata)
            .expect("Failed to get boss token account after deposit");

        self.offer_vault_deposit_invariant(
            boss_before_deposit,
            boss_after_deposit,
            vault_token_account_before_deposit,
            vault_token_account_after_deposit,
            deposit_amount,
        );
    }

    fn withdraw_from_vault(
        &mut self,
        withdraw_amount: u64,
        token_mint: Pubkey,
        from: Pubkey,
        message: Option<&str>,
        token_program_id: Pubkey,
    ) {
        let offer_vault_authority = self
            .fuzz_accounts
            .offer_vault_authority
            .get(&mut self.trident)
            .expect("Offer Vault Authority Storage empty");

        let boss_ata =
            self.trident
                .get_associated_token_address(&token_mint, &from, &token_program_id);

        let vault_token_account = self.trident.get_associated_token_address(
            &token_mint,
            &offer_vault_authority,
            &token_program_id,
        );

        let boss = self
            .fuzz_accounts
            .boss
            .get(&mut self.trident)
            .expect("Boss Storage empty");
        let state = self
            .fuzz_accounts
            .state
            .get(&mut self.trident)
            .expect("State Storage empty");

        let boss_before_withdraw = self
            .trident
            .get_token_account(boss_ata)
            .expect("Failed to get boss token account before withdraw");

        let vault_token_account_before_withdraw = self
            .trident
            .get_token_account(vault_token_account)
            .expect("Failed to get vault token account before withdraw");

        let vault_withdraw = types::onreapp::OfferVaultWithdrawInstruction::data(
            onreapp::OfferVaultWithdrawInstructionData::new(withdraw_amount),
        )
        .accounts(onreapp::OfferVaultWithdrawInstructionAccounts::new(
            offer_vault_authority,
            token_mint,
            boss_ata,
            vault_token_account,
            boss,
            state,
            token_program_id,
        ))
        .instruction();

        let res = self.trident.process_transaction(&[vault_withdraw], message);

        assert!(
            res.is_success(),
            "Failed to withdraw from vault: logs: {:#?}",
            res.logs()
        );

        let boss_after_withdraw = self
            .trident
            .get_token_account(boss_ata)
            .expect("Failed to get boss token account after withdraw");

        let vault_token_account_after_withdraw = self
            .trident
            .get_token_account(vault_token_account)
            .expect("Failed to get vault token account after withdraw");

        self.offer_vault_withdraw_invariant(
            boss_before_withdraw,
            boss_after_withdraw,
            vault_token_account_before_withdraw,
            vault_token_account_after_withdraw,
            withdraw_amount,
        );
    }

    fn setup_onyc_mint_and_initialize(
        &mut self,
        payer: Pubkey,
        boss: Pubkey,
        mint_authority: Pubkey,
    ) {
        let state = self.fuzz_accounts.state.insert(
            &mut self.trident,
            Some(PdaSeeds::new(&[b"state"], types::onreapp::program_id())),
        );

        let onyc_mint = self.fuzz_accounts.onyc_mint.insert(&mut self.trident, None);

        let ixs =
            self.trident
                .initialize_mint_2022(&payer, &onyc_mint, 8, &mint_authority, None, &[]);

        let res = self.trident.process_transaction(&ixs, None);
        assert!(
            res.is_success(),
            "Failed to initialize ONyc mint: logs: {:#?}",
            res.logs()
        );

        let offer_mint_authority = self.fuzz_accounts.offer_mint_authority.insert(
            &mut self.trident,
            Some(PdaSeeds::new(
                &[b"mint_authority"],
                types::onreapp::program_id(),
            )),
        );

        let offer_vault_authority = self.fuzz_accounts.offer_vault_authority.insert(
            &mut self.trident,
            Some(PdaSeeds::new(
                &[b"offer_vault_authority"],
                types::onreapp::program_id(),
            )),
        );

        let program_data_address = self
            .trident
            .get_program_data_address_v3(&onreapp::program_id());

        let initialize =
            types::onreapp::InitializeInstruction::data(InitializeInstructionData::new())
                .accounts(InitializeInstructionAccounts::new(
                    state,
                    offer_mint_authority,
                    offer_vault_authority,
                    boss,
                    program_data_address,
                    onyc_mint,
                ))
                .instruction();

        let res = self
            .trident
            .process_transaction(&[initialize], Some("initialize"));

        assert!(
            res.is_success(),
            "Failed to initialize: logs: {:#?}",
            res.logs()
        );

        self.initialize_invariant(state, boss, onyc_mint);
    }

    fn setup_permissionless_authority(&mut self, name: String) {
        let state = self
            .fuzz_accounts
            .state
            .get(&mut self.trident)
            .expect("State Storage empty");
        let boss = self
            .fuzz_accounts
            .boss
            .get(&mut self.trident)
            .expect("Boss Storage empty");
        let permissionless_authority = self.fuzz_accounts.permissionless_authority.insert(
            &mut self.trident,
            Some(PdaSeeds::new(
                &[b"permissionless-1"],
                types::onreapp::program_id(),
            )),
        );
        let initialize_permissionless_authority =
            onreapp::InitializePermissionlessAuthorityInstruction::data(
                onreapp::InitializePermissionlessAuthorityInstructionData::new(name.clone()),
            )
            .accounts(
                onreapp::InitializePermissionlessAuthorityInstructionAccounts::new(
                    permissionless_authority,
                    state,
                    boss,
                ),
            )
            .instruction();

        let res = self.trident.process_transaction(
            &[initialize_permissionless_authority],
            Some("initialize_permissionless_authority"),
        );

        assert!(
            res.is_success(),
            "Failed to initialize permissionless authority: logs: {:#?}",
            res.logs()
        );

        self.initialize_permissionless_authority_invariant(permissionless_authority, name);
    }
}

fn main() {
    FuzzTest::fuzz(1000, 50);
}
