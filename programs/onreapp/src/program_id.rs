use anchor_lang::prelude::Pubkey;
use solana_program::pubkey;


#[cfg(all(feature = "mainnet", feature = "mainnet-test"))]
compile_error!("'mainnet' and 'mainnet-test' features are mutually exclusive. Have you missed to disable default features?");

#[cfg(all(feature = "mainnet", feature = "devnet-test"))]
compile_error!("'mainnet' and 'devnet-test' features are mutually exclusive. Have you missed to disable default features?");

#[cfg(all(feature = "mainnet", feature = "devnet-dev"))]
compile_error!("'mainnet' and 'devnet-dev' features are mutually exclusive. Have you missed to disable default features?");

#[cfg(all(feature = "mainnet-test", feature = "devnet-test"))]
compile_error!("'mainnet-test' and 'devnet-test' features are mutually exclusive. Have you missed to disable default features?");

#[cfg(all(feature = "mainnet-test", feature = "devnet-dev"))]
compile_error!("'mainnet-test' and 'devnet-dev' features are mutually exclusive. Have you missed to disable default features?");

#[cfg(all(feature = "devnet-test", feature = "devnet-dev"))]
compile_error!("'devnet-test' and 'devnet-dev' features are mutually exclusive. Have you missed to disable default features?");

pub const ONRE_MAINNET_PROGRAM_ID: Pubkey = pubkey!("onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe");
pub const ONRE_MAINNET_TEST_PROGRAM_ID: Pubkey = pubkey!("J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2");
pub const ONRE_DEVNET_TEST_PROGRAM_ID: Pubkey = pubkey!("J24jWEosQc5jgkdPm3YzNgzQ54CqNKkhzKy56XXJsLo2");
pub const ONRE_DEVNET_DEV_PROGRAM_ID: Pubkey = pubkey!("devHfQHgiFNifkLW49RCXpyTUZMyKuBNnFSbrQ8XsbX");

#[cfg(feature = "mainnet")]
pub const ONRE_PROGRAM_ID: Pubkey = ONRE_MAINNET_PROGRAM_ID;

#[cfg(feature = "mainnet-test")]
pub const ONRE_PROGRAM_ID: Pubkey = ONRE_MAINNET_TEST_PROGRAM_ID;

#[cfg(feature = "devnet-test")]
pub const ONRE_PROGRAM_ID: Pubkey = ONRE_DEVNET_TEST_PROGRAM_ID;

#[cfg(feature = "devnet-dev")]
pub const ONRE_PROGRAM_ID: Pubkey = ONRE_DEVNET_DEV_PROGRAM_ID;

