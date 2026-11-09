# VaultConfig custody operations

`deposit_vault_config(amount)` and `withdraw_vault_config(amount)` move existing
SPL Token or Token-2022 balances. They do not mint, burn, change BUFFER accrual
state, create configuration, or change withdrawal destinations. Both instructions
are blocked by the global kill switch and reject active transfer-fee mints.
The generated IDL/types expose these instructions; CLI commands are not added.

## Deposits

Any signer may deposit a positive amount from their canonical mint ATA into an
existing VaultConfig's canonical mint ATA. The depositor funds ATA creation when
needed. Zero is rejected. Generic VaultConfig instances can hold multiple mints,
each in its own ATA. Token-specific BUFFER vaults accept only their managed mint
through these instructions.

Provide `vault`, `mint`, `depositor_token_account`, `vault_token_account`,
`depositor`, `state`, `token_program`, `associated_token_program`, and
`system_program`. `VaultConfigDepositedEvent` records vault, mint, depositor, and
the transferred amount.

## Withdrawals

Every withdrawal pays the selected mint's ATA of the stored
`vault.withdrawal_destination`. This recipient does not gain signing authority
over the vault. A caller cannot redirect a withdrawal to another owner.

| VaultConfig kind | Required caller |
| --- | --- |
| Fee | Any signer |
| Proceeds | Any signer |
| Liquidity | Current `State::boss` |
| BufferReserve | Current `State::boss` |

The caller pays for creating the destination ATA when needed. A positive amount
withdraws exactly that amount; zero withdraws the whole current balance. Empty
vaults and amounts exceeding the balance fail. Failed operations roll back token
movement and ATA creation.

Provide `vault`, `mint`, `vault_token_account`, `destination`,
`destination_token_account`, `caller`, `state`, `token_program`,
`associated_token_program`, and `system_program`. `VaultConfigWithdrawnEvent`
records vault, mint, caller, fixed destination, and actual transferred amount.

## Authority and signing seeds

The OnRe program signs as the selected custody authority. Configuration bodies,
canonical PDA addresses and stored bumps are validated before using their seeds.
Both new custody operations and modular liquidity payouts use the shared resolver
in `utils/vault_authority.rs`.

| Custody identity | Signing seeds before the bump |
| --- | --- |
| Generic VaultConfig | `"vault", kind_u8, instance_id_u64_le` |
| Managed-token BUFFER reserve | `"buffer_reserve", managed_mint` |
| Managed-token BUFFER management fee | `"buffer_management", managed_mint` |
| Managed-token BUFFER performance fee | `"buffer_performance", managed_mint` |
| Legacy ConfigurableVault | `"configurable_vault", legacy_kind_seed` |
| Legacy redemption authority | `REDEMPTION_OFFER_VAULT_AUTHORITY` |

The new deposit/withdraw instructions accept only VaultConfig accounts. Legacy
configurable vaults and redemption custody retain their existing movement
instructions and access policies. Modular offers may explicitly select a generic
Liquidity VaultConfig, a legacy ConfigurableVault, or the canonical redemption
authority. Only that selected authority's asset ATA supplies payout liquidity.
Unselected old funds are not added to the balance.

Selecting the old redemption authority shares the actual old balance: a payout
through either execution path reduces the funds available to the other. It does
not merge quote parameters or Prop AMM pressure state. Both NAV and modular Prop
AMM payouts can use this custody adapter. Buy-side refill targets are supported on
generic Liquidity VaultConfig instances; legacy custody has no modular target.
See [modular Prop AMM](MODULAR_PROP_AMM.md) for pricing and pressure accounting. ONyc continues using its existing singleton BUFFER accounts.
