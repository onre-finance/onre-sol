/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/onreapp.json`.
 */
export type Onreapp = {
  "address": "onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe",
  "metadata": {
    "name": "onreapp",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "docs": [
    "The main program module for the Onre App.",
    "",
    "This module defines the entry points for all program instructions. It facilitates the creation",
    "and management of offers where a \"boss\" provides one or two types of buy tokens in exchange for",
    "sell tokens. A key feature is the dynamic pricing model for offers, where the amount of",
    "sell token required can change over the offer's duration based on predefined parameters.",
    "",
    "Core functionalities include:",
    "- Making offers with dynamic pricing (`make_offer_one`, `make_offer_two`).",
    "- Taking offers, respecting the current price (`take_offer_one`, `take_offer_two`).",
    "- Closing offers (`close_offer_one`, `close_offer_two`).",
    "- Program state initialization and boss management (`initialize`, `set_boss`).",
    "",
    "# Dynamic Pricing Model",
    "The price (amount of sell tokens per buy token) is determined by:",
    "- `sell_token_start_amount`: Sell token amount at the beginning of the offer.",
    "- `sell_token_end_amount`: Sell token amount at the end of the offer.",
    "- `offer_start_time`, `offer_end_time`: Defines the offer's active duration.",
    "- `price_fix_duration`: The duration of each discrete pricing interval within the offer period.",
    "The price interpolates linearly across these intervals.",
    "",
    "# Security",
    "- Access controls are enforced, for example, ensuring only the `boss` can create offers or update critical state.",
    "- PDA (Program Derived Address) accounts are used for offer and token authorities, ensuring ownership.",
    "- Events are emitted for significant actions (e.g., `OfferMadeOne`, `OfferTakenTwo`) for off-chain traceability."
  ],
  "instructions": [
    {
      "name": "addAdmin",
      "docs": [
        "Adds a new admin to the admin state.",
        "",
        "Delegates to `admin::add_admin` to add a new admin to the admin list.",
        "Only the boss can call this instruction to add new admins.",
        "# Arguments",
        "- `ctx`: Context for `AddAdmin`.",
        "- `new_admin`: Public key of the new admin to be added."
      ],
      "discriminator": [
        177,
        236,
        33,
        205,
        124,
        152,
        55,
        186
      ],
      "accounts": [
        {
          "name": "adminState",
          "docs": [
            "Admin state account containing the list of admins."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  100,
                  109,
                  105,
                  110,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "boss",
          "docs": [
            "The signer authorizing the addition, must be the boss."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "newAdmin",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "addOfferVector",
      "docs": [
        "Adds a time vector to an existing offer.",
        "",
        "Delegates to `offer::add_offer_time_vector`.",
        "Creates a new time vector with auto-generated vector_id for the specified offer.",
        "Emits a `OfferVectorAdded` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `AddOfferVector`.",
        "- `offer_id`: ID of the offer to add the vector to.",
        "- `base_time`: Unix timestamp when the vector becomes active.",
        "- `base_price`: Price at the beginning of the vector.",
        "- `apr`: Annual Percentage Rate (APR) (see OfferVector::apr for details).",
        "- `price_fix_duration`: Duration in seconds for each price interval."
      ],
      "discriminator": [
        198,
        139,
        180,
        6,
        156,
        171,
        188,
        61
      ],
      "accounts": [
        {
          "name": "offerAccount",
          "docs": [
            "The offer account containing all offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The signer authorizing the time vector addition (must be boss)."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "offerId",
          "type": "u64"
        },
        {
          "name": "baseTime",
          "type": "u64"
        },
        {
          "name": "basePrice",
          "type": "u64"
        },
        {
          "name": "apr",
          "type": "u64"
        },
        {
          "name": "priceFixDuration",
          "type": "u64"
        }
      ]
    },
    {
      "name": "closeOffer",
      "docs": [
        "Closes a offer.",
        "",
        "Delegates to `offer::close_offer`.",
        "Removes the offer from the offers account and clears its data.",
        "Emits a `CloseOfferEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `CloseOffer`.",
        "- `offer_id`: ID of the offer to close."
      ],
      "discriminator": [
        191,
        72,
        67,
        35,
        239,
        209,
        97,
        132
      ],
      "accounts": [
        {
          "name": "offerAccount",
          "docs": [
            "The offer account within the OfferAccount, rent paid by `boss`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "boss",
          "docs": [
            "The signer funding and authorizing the offer closure."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account creation and rent payment."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "offerId",
          "type": "u64"
        }
      ]
    },
    {
      "name": "deleteOfferVector",
      "docs": [
        "Deletes a time vector from an offer.",
        "",
        "Delegates to `offer::delete_offer_vector`.",
        "Removes the specified time vector from the offer by setting it to default values.",
        "Only the boss can delete time vectors from offers.",
        "Emits a `OfferVectorDeleted` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `DeleteOfferVector`.",
        "- `offer_id`: ID of the offer containing the vector to delete.",
        "- `vector_id`: ID of the vector to delete."
      ],
      "discriminator": [
        87,
        40,
        79,
        151,
        78,
        121,
        46,
        159
      ],
      "accounts": [
        {
          "name": "offerAccount",
          "docs": [
            "The offer account containing all offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The signer authorizing the time vector deletion (must be boss)."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "offerId",
          "type": "u64"
        },
        {
          "name": "vectorId",
          "type": "u64"
        }
      ]
    },
    {
      "name": "initialize",
      "docs": [
        "Initializes the program state.",
        "",
        "Delegates to `initialize::initialize` to set the initial boss in the state account."
      ],
      "discriminator": [
        175,
        175,
        109,
        31,
        13,
        152,
        155,
        237
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "The program state account, initialized with the boss’s public key.",
            "",
            "# Note",
            "- Space is allocated as `8 + State::INIT_SPACE` bytes, where 8 bytes are for the discriminator.",
            "- Seeded with `\"state\"` and a bump for PDA derivation."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "boss",
          "docs": [
            "The signer funding and authorizing the state initialization, becomes the boss."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account creation and rent payment."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "initializeAdminState",
      "docs": [
        "Initializes the admin state.",
        "",
        "Delegates to `initialize_admin_state::initialize_admin_state` to set up the admin state account.",
        "Only the boss can call this instruction to create the admin state account.",
        "# Arguments",
        "- `ctx`: Context for `InitializeAdminState`."
      ],
      "discriminator": [
        143,
        116,
        15,
        14,
        59,
        122,
        82,
        86
      ],
      "accounts": [
        {
          "name": "adminState",
          "docs": [
            "Program authority account to be initialized."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  100,
                  109,
                  105,
                  110,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The boss paying for account creation and authorizing the initialization."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account creation."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "initializeKillSwitchState",
      "docs": [
        "Initializes the kill switch state.",
        "",
        "Delegates to `initialization::initialize_kill_switch_state` to set up the kill switch state account.",
        "The kill switch is initialized in the disabled (false) state by default.",
        "Only the boss can call this instruction to create the kill switch state account.",
        "",
        "# Arguments",
        "- `ctx`: Context for `InitializeKillSwitchState`."
      ],
      "discriminator": [
        254,
        35,
        214,
        241,
        136,
        217,
        22,
        155
      ],
      "accounts": [
        {
          "name": "killSwitchState",
          "docs": [
            "Program authority account to be initialized."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  107,
                  105,
                  108,
                  108,
                  95,
                  115,
                  119,
                  105,
                  116,
                  99,
                  104
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The boss paying for account creation and authorizing the initialization."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account creation."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "initializeOffers",
      "docs": [
        "Initializes the offers account.",
        "",
        "Delegates to `offer::initialize_offers`.",
        "Only the boss can call this instruction to create the offers account.",
        "",
        "# Arguments",
        "- `ctx`: Context for `InitializeOffers`."
      ],
      "discriminator": [
        226,
        123,
        68,
        141,
        151,
        152,
        119,
        113
      ],
      "accounts": [
        {
          "name": "offerAccount",
          "docs": [
            "The buy offer account to initialize, rent paid by `boss`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "boss",
          "docs": [
            "The signer authorizing the initialization, must be the boss."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account creation and rent payment."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "initializePermissionlessAccount",
      "docs": [
        "Initializes a permissionless account.",
        "",
        "Delegates to `initialize::initialize_permissionless_account` to create a new permissionless account.",
        "The account is created as a PDA with the seed \"permissionless-1\".",
        "Only the boss can initialize permissionless accounts."
      ],
      "discriminator": [
        144,
        160,
        10,
        56,
        91,
        17,
        77,
        115
      ],
      "accounts": [
        {
          "name": "permissionlessAccount",
          "docs": [
            "The permissionless account to be created.",
            "",
            "# Note",
            "- Space is allocated as `8 + PermissionlessAccount::INIT_SPACE` bytes",
            "- Seeded with hardcoded \"permissionless-1\" for PDA derivation"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  101,
                  114,
                  109,
                  105,
                  115,
                  115,
                  105,
                  111,
                  110,
                  108,
                  101,
                  115,
                  115,
                  45,
                  49
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "The program state account, used to verify boss authorization."
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The boss account that authorizes and pays for the permissionless account creation."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account creation and rent payment."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "name",
          "type": "string"
        }
      ]
    },
    {
      "name": "initializeVaultAuthority",
      "discriminator": [
        47,
        125,
        11,
        209,
        248,
        240,
        52,
        77
      ],
      "accounts": [
        {
          "name": "buyOfferVaultAuthority",
          "docs": [
            "The buy offer vault authority account to initialize, rent paid by `boss`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The signer authorizing the initialization, must be the boss."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account creation and rent payment."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "killSwitch",
      "docs": [
        "Enables or disables the kill switch.",
        "",
        "Delegates to `kill_switch::kill_switch` to change the kill switch state.",
        "When enabled (true), the kill switch can halt critical program operations.",
        "When disabled (false), normal program operations can proceed.",
        "",
        "Access control:",
        "- Both boss and admins can enable the kill switch",
        "- Only the boss can disable the kill switch",
        "",
        "# Arguments",
        "- `ctx`: Context for `KillSwitch`.",
        "- `enable`: True to enable the kill switch, false to disable it."
      ],
      "discriminator": [
        189,
        76,
        222,
        157,
        130,
        131,
        241,
        144
      ],
      "accounts": [
        {
          "name": "killSwitchState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  107,
                  105,
                  108,
                  108,
                  95,
                  115,
                  119,
                  105,
                  116,
                  99,
                  104
                ]
              }
            ]
          }
        },
        {
          "name": "adminState",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  100,
                  109,
                  105,
                  110,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "signer",
          "signer": true
        }
      ],
      "args": [
        {
          "name": "enable",
          "type": "bool"
        }
      ]
    },
    {
      "name": "makeOffer",
      "docs": [
        "Creates an offer.",
        "",
        "Delegates to `offer::make_offer`.",
        "The price of the token_out changes over time based on `base_price`,",
        "`end_price`, and `price_fix_duration` within the offer's active time window.",
        "Emits a `OfferMade` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `MakeOffer`.",
        "- `fee_basis_points`: Fee in basis points (e.g., 500 = 5%) charged when taking the offer."
      ],
      "discriminator": [
        214,
        98,
        97,
        35,
        59,
        12,
        44,
        178
      ],
      "accounts": [
        {
          "name": "offerAccount",
          "docs": [
            "The offer account within the OfferAccount, rent paid by `boss`. Already initialized in initialize."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "The offer vault authority PDA that controls vault token accounts"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "Mint of the token_in for the offer."
          ]
        },
        {
          "name": "tokenInProgram"
        },
        {
          "name": "vaultTokenInAccount",
          "docs": [
            "Vault token_in account, used to transfer tokens to a program owned account for burning",
            "when program has mint authority."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "vaultAuthority"
              },
              {
                "kind": "account",
                "path": "tokenInProgram"
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "Mint of the token_out for the offer."
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The signer funding and authorizing the offer creation."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "associatedTokenProgram",
          "docs": [
            "Associated Token Program for automatic token account creation"
          ],
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account creation and rent payment."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "feeBasisPoints",
          "type": "u64"
        }
      ]
    },
    {
      "name": "offerVaultDeposit",
      "docs": [
        "Deposits tokens into the offer vault.",
        "",
        "Delegates to `vault_operations::offer_vault_deposit`.",
        "Transfers tokens from boss's account to offer vault's token account for the specified mint.",
        "Creates vault token account if it doesn't exist using init_if_needed.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `OfferVaultDeposit`.",
        "- `amount`: Amount of tokens to deposit."
      ],
      "discriminator": [
        69,
        131,
        100,
        85,
        82,
        151,
        72,
        74
      ],
      "accounts": [
        {
          "name": "vaultAuthority",
          "docs": [
            "The offer vault authority account that controls the vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "tokenMint",
          "docs": [
            "The token mint for the deposit."
          ]
        },
        {
          "name": "bossTokenAccount",
          "docs": [
            "Boss's token account for the specific mint (source of tokens)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "boss"
              },
              {
                "kind": "account",
                "path": "tokenProgram"
              },
              {
                "kind": "account",
                "path": "tokenMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "vaultTokenAccount",
          "docs": [
            "Vault's token account for the specific mint (destination of tokens).",
            "Uses init_if_needed to create the account if it doesn't exist."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "vaultAuthority"
              },
              {
                "kind": "account",
                "path": "tokenProgram"
              },
              {
                "kind": "account",
                "path": "tokenMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "boss",
          "docs": [
            "The signer authorizing the deposit, must be the boss."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program."
          ]
        },
        {
          "name": "associatedTokenProgram",
          "docs": [
            "Associated Token program."
          ],
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account creation and rent payment."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "offerVaultWithdraw",
      "docs": [
        "Withdraws tokens from the offer vault.",
        "",
        "Delegates to `vault_operations::offer_vault_withdraw`.",
        "Transfers tokens from offer vault's token account to boss's account for the specified mint.",
        "Creates boss token account if it doesn't exist using init_if_needed.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `OfferVaultWithdraw`.",
        "- `amount`: Amount of tokens to withdraw."
      ],
      "discriminator": [
        230,
        135,
        129,
        48,
        138,
        202,
        20,
        72
      ],
      "accounts": [
        {
          "name": "vaultAuthority",
          "docs": [
            "The offer vault authority account that controls the vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "tokenMint",
          "docs": [
            "The token mint for the withdrawal."
          ]
        },
        {
          "name": "bossTokenAccount",
          "docs": [
            "Boss's token account for the specific mint (destination of tokens)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "boss"
              },
              {
                "kind": "account",
                "path": "tokenProgram"
              },
              {
                "kind": "account",
                "path": "tokenMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "vaultTokenAccount",
          "docs": [
            "Vault's token account for the specific mint (source of tokens)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "vaultAuthority"
              },
              {
                "kind": "const",
                "value": [
                  6,
                  221,
                  246,
                  225,
                  215,
                  101,
                  161,
                  147,
                  217,
                  203,
                  225,
                  70,
                  206,
                  235,
                  121,
                  172,
                  28,
                  180,
                  133,
                  237,
                  95,
                  91,
                  55,
                  145,
                  58,
                  140,
                  245,
                  133,
                  126,
                  255,
                  0,
                  169
                ]
              },
              {
                "kind": "account",
                "path": "tokenMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "boss",
          "docs": [
            "The signer authorizing the withdrawal, must be the boss."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program."
          ]
        },
        {
          "name": "associatedTokenProgram",
          "docs": [
            "Associated Token program."
          ],
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account creation and rent payment."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "removeAdmin",
      "docs": [
        "Removes an admin from the admin state.",
        "",
        "Delegates to `admin::remove_admin` to remove an admin from the admin list.",
        "Only the boss can call this instruction to remove admins.",
        "# Arguments",
        "- `ctx`: Context for `RemoveAdmin`.",
        "- `admin_to_remove`: Public key of the admin to be removed."
      ],
      "discriminator": [
        74,
        202,
        71,
        106,
        252,
        31,
        72,
        183
      ],
      "accounts": [
        {
          "name": "adminState",
          "docs": [
            "Admin state account containing the list of admins."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  100,
                  109,
                  105,
                  110,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "boss",
          "docs": [
            "The boss calling this function."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "adminToRemove",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "setBoss",
      "docs": [
        "Delegates to `set_boss::set_boss` to change the boss, emitting a `BossUpdated` event."
      ],
      "discriminator": [
        144,
        141,
        235,
        104,
        167,
        250,
        41,
        54
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "The program state account, containing the current boss to be updated.",
            "",
            "# Constraints",
            "- Must be mutable to allow updating the `boss` field.",
            "- The `has_one = boss` constraint ensures only the current boss can modify it."
          ],
          "writable": true
        },
        {
          "name": "boss",
          "docs": [
            "The current boss, signing the transaction to authorize the update."
          ],
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program, included for potential rent accounting."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "newBoss",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "takeOffer",
      "docs": [
        "Takes a offer.",
        "",
        "Delegates to `offer::take_offer`.",
        "Allows a user to exchange token_in for token_out based on the offer's dynamic price.",
        "Emits a `TakeOfferEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `TakeOffer`.",
        "- `offer_id`: ID of the offer to take.",
        "- `token_in_amount`: Amount of token_in to provide."
      ],
      "discriminator": [
        128,
        156,
        242,
        207,
        237,
        192,
        103,
        240
      ],
      "accounts": [
        {
          "name": "offerAccount",
          "docs": [
            "The offer account containing all active offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing the boss public key"
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The boss account that receives token_in payments",
            "Must match the boss stored in the program state"
          ]
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "The offer vault authority PDA that controls vault token accounts"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "vaultTokenInAccount",
          "docs": [
            "Vault's token_in account, used for burning tokens when program has mint authority"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "vaultAuthority"
              },
              {
                "kind": "account",
                "path": "tokenInProgram"
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "vaultTokenOutAccount",
          "docs": [
            "Vault's token_out account (source of tokens to distribute to user)"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "vaultAuthority"
              },
              {
                "kind": "account",
                "path": "tokenOutProgram"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The mint account for the input token (what user pays)",
            "Must be mutable to allow burning when program has mint authority"
          ],
          "writable": true
        },
        {
          "name": "tokenInProgram"
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "The mint account for the output token (what user receives)",
            "Must be mutable to allow minting when program has mint authority"
          ],
          "writable": true
        },
        {
          "name": "tokenOutProgram"
        },
        {
          "name": "userTokenInAccount",
          "docs": [
            "User's token_in account (source of payment)"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "user"
              },
              {
                "kind": "account",
                "path": "tokenInProgram"
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "userTokenOutAccount",
          "docs": [
            "User's token_out account (destination of received tokens)",
            "Uses init_if_needed to automatically create account if it doesn't exist"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "user"
              },
              {
                "kind": "account",
                "path": "tokenOutProgram"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "bossTokenInAccount",
          "docs": [
            "Boss's token_in account (destination of user's payment)"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "boss"
              },
              {
                "kind": "account",
                "path": "tokenInProgram"
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "mintAuthorityPda",
          "docs": [
            "Mint authority PDA for direct minting (when program has mint authority)"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  109,
                  105,
                  110,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "killSwitchState",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  107,
                  105,
                  108,
                  108,
                  95,
                  115,
                  119,
                  105,
                  116,
                  99,
                  104
                ]
              }
            ]
          }
        },
        {
          "name": "user",
          "docs": [
            "The user taking the offer (must sign the transaction)"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "associatedTokenProgram",
          "docs": [
            "Associated Token Program for automatic token account creation"
          ],
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program required for account creation"
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "offerId",
          "type": "u64"
        },
        {
          "name": "tokenInAmount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "takeOfferPermissionless",
      "docs": [
        "Takes a offer using permissionless flow with intermediary accounts.",
        "",
        "Delegates to `offer::take_offer_permissionless`.",
        "Similar to take_offer but routes token transfers through intermediary accounts",
        "owned by the program instead of direct user-to-boss and vault-to-user transfers.",
        "Emits a `TakeOfferPermissionlessEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `TakeOfferPermissionless`.",
        "- `offer_id`: ID of the offer to take.",
        "- `token_in_amount`: Amount of token_in to provide."
      ],
      "discriminator": [
        37,
        190,
        224,
        77,
        197,
        39,
        203,
        230
      ],
      "accounts": [
        {
          "name": "offerAccount",
          "docs": [
            "The offer account containing all active offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing the boss public key"
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The boss account that receives token_in payments",
            "Must match the boss stored in the program state"
          ],
          "relations": [
            "state"
          ]
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "The offer vault authority PDA that controls vault token accounts"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "vaultTokenInAccount",
          "docs": [
            "Vault's token_in account, used for burning tokens when program has mint authority"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "vaultAuthority"
              },
              {
                "kind": "account",
                "path": "tokenInProgram"
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "vaultTokenOutAccount",
          "docs": [
            "Vault's token_out account (source of tokens to distribute, when program doesn't have mint authority)"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "vaultAuthority"
              },
              {
                "kind": "account",
                "path": "tokenOutProgram"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "permissionlessAuthority",
          "docs": [
            "The permissionless authority PDA that controls intermediary token accounts"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  101,
                  114,
                  109,
                  105,
                  115,
                  115,
                  105,
                  111,
                  110,
                  108,
                  101,
                  115,
                  115,
                  45,
                  49
                ]
              }
            ]
          }
        },
        {
          "name": "permissionlessTokenInAccount",
          "docs": [
            "Permissionless intermediary token_in account (temporary holding for token_in)"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "permissionlessAuthority"
              },
              {
                "kind": "account",
                "path": "tokenInProgram"
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "permissionlessTokenOutAccount",
          "docs": [
            "Permissionless intermediary token_out account (temporary holding for token_out)"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "permissionlessAuthority"
              },
              {
                "kind": "account",
                "path": "tokenOutProgram"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The mint account for the input token (what user pays)",
            "Must be mutable to allow burning when program has mint authority"
          ],
          "writable": true
        },
        {
          "name": "tokenInProgram"
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "The mint account for the output token (what user receives)",
            "Must be mutable to allow minting when program has mint authority"
          ],
          "writable": true
        },
        {
          "name": "tokenOutProgram"
        },
        {
          "name": "userTokenInAccount",
          "docs": [
            "User's token_in account (source of payment)"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "user"
              },
              {
                "kind": "account",
                "path": "tokenInProgram"
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "userTokenOutAccount",
          "docs": [
            "User's token_out account (destination of received tokens)"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "user"
              },
              {
                "kind": "account",
                "path": "tokenOutProgram"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "bossTokenInAccount",
          "docs": [
            "Boss's token_in account (final destination of user's payment)"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "boss"
              },
              {
                "kind": "account",
                "path": "tokenInProgram"
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "mintAuthorityPda",
          "docs": [
            "Mint authority PDA for direct minting (when program has mint authority)"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  109,
                  105,
                  110,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "killSwitchState",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  107,
                  105,
                  108,
                  108,
                  95,
                  115,
                  119,
                  105,
                  116,
                  99,
                  104
                ]
              }
            ]
          }
        },
        {
          "name": "user",
          "docs": [
            "The user taking the offer (must sign the transaction)"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "associatedTokenProgram",
          "docs": [
            "Associated Token Program for automatic token account creation"
          ],
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program required for account creation"
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "offerId",
          "type": "u64"
        },
        {
          "name": "tokenInAmount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "transferMintAuthorityToBoss",
      "docs": [
        "Transfers mint authority from a program-derived PDA back to the boss.",
        "",
        "Delegates to `mint_authority::transfer_mint_authority_to_boss`.",
        "Only the boss can call this instruction to recover mint authority for a specific token.",
        "This serves as an emergency recovery mechanism.",
        "Emits a `MintAuthorityTransferredToBossEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `TransferMintAuthorityToBoss`."
      ],
      "discriminator": [
        197,
        61,
        42,
        52,
        70,
        93,
        30,
        125
      ],
      "accounts": [
        {
          "name": "boss",
          "docs": [
            "The current boss account, must sign the transaction",
            "Must match the boss stored in the program state"
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state containing the current boss public key"
          ]
        },
        {
          "name": "mint",
          "docs": [
            "The token mint whose authority will be transferred back to boss",
            "Must currently have the program PDA as its mint authority"
          ],
          "writable": true
        },
        {
          "name": "mintAuthorityPda",
          "docs": [
            "Program-derived account that currently holds mint authority",
            "Must be derived from [MINT_AUTHORITY, mint_pubkey] and currently be the mint authority"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  109,
                  105,
                  110,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "docs": [
            "SPL Token program for mint authority operations"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "transferMintAuthorityToProgram",
      "docs": [
        "Transfers mint authority from the boss to a program-derived PDA.",
        "",
        "Delegates to `mint_authority::transfer_mint_authority_to_program`.",
        "Only the boss can call this instruction to transfer mint authority for a specific token.",
        "The PDA is derived from the mint address and can later be used to mint tokens.",
        "Emits a `MintAuthorityTransferredToProgramEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `TransferMintAuthorityToProgram`."
      ],
      "discriminator": [
        98,
        112,
        50,
        135,
        53,
        6,
        149,
        232
      ],
      "accounts": [
        {
          "name": "boss",
          "docs": [
            "The current boss account, must sign the transaction",
            "Must match the boss stored in the program state"
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state containing the current boss public key"
          ]
        },
        {
          "name": "mint",
          "docs": [
            "The token mint whose authority will be transferred",
            "Must currently have the boss as its mint authority"
          ],
          "writable": true
        },
        {
          "name": "mintAuthorityPda",
          "docs": [
            "Program-derived account that will become the new mint authority",
            "Derived from [MINT_AUTHORITY, mint_pubkey] to ensure uniqueness per token"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  109,
                  105,
                  110,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104,
                  111,
                  114,
                  105,
                  116,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "docs": [
            "SPL Token program for mint authority operations"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "updateOfferFee",
      "docs": [
        "Updates the fee basis points for an offer.",
        "",
        "Delegates to `offer::update_offer_fee`.",
        "Allows the boss to modify the fee charged when users take the offer.",
        "Emits a `OfferFeeUpdatedEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `UpdateOfferFee`.",
        "- `offer_id`: ID of the offer to update.",
        "- `new_fee_basis_points`: New fee in basis points (0-10000)."
      ],
      "discriminator": [
        254,
        162,
        70,
        248,
        117,
        70,
        197,
        118
      ],
      "accounts": [
        {
          "name": "offerAccount",
          "docs": [
            "The offer account containing all offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  111,
                  102,
                  102,
                  101,
                  114,
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The signer authorizing the fee update (must be boss)."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "offerId",
          "type": "u64"
        },
        {
          "name": "newFeeBasisPoints",
          "type": "u64"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "adminState",
      "discriminator": [
        190,
        42,
        124,
        96,
        242,
        52,
        141,
        28
      ]
    },
    {
      "name": "killSwitchState",
      "discriminator": [
        254,
        243,
        235,
        232,
        86,
        55,
        45,
        62
      ]
    },
    {
      "name": "offerAccount",
      "discriminator": [
        152,
        98,
        9,
        183,
        115,
        190,
        31,
        201
      ]
    },
    {
      "name": "offerVaultAuthority",
      "discriminator": [
        68,
        19,
        219,
        165,
        51,
        111,
        201,
        255
      ]
    },
    {
      "name": "permissionlessAccount",
      "discriminator": [
        9,
        107,
        135,
        228,
        163,
        199,
        67,
        169
      ]
    },
    {
      "name": "state",
      "discriminator": [
        216,
        146,
        107,
        94,
        104,
        75,
        182,
        177
      ]
    }
  ],
  "events": [
    {
      "name": "bossUpdated",
      "discriminator": [
        240,
        140,
        218,
        236,
        20,
        65,
        191,
        69
      ]
    },
    {
      "name": "closeOfferEvent",
      "discriminator": [
        254,
        189,
        199,
        59,
        229,
        2,
        175,
        126
      ]
    },
    {
      "name": "mintAuthorityTransferredToBossEvent",
      "discriminator": [
        86,
        223,
        255,
        189,
        210,
        62,
        212,
        151
      ]
    },
    {
      "name": "mintAuthorityTransferredToProgramEvent",
      "discriminator": [
        237,
        15,
        101,
        27,
        85,
        70,
        173,
        232
      ]
    },
    {
      "name": "offerFeeUpdatedEvent",
      "discriminator": [
        65,
        77,
        241,
        6,
        23,
        133,
        45,
        180
      ]
    },
    {
      "name": "offerMadeEvent",
      "discriminator": [
        206,
        97,
        61,
        193,
        90,
        177,
        83,
        200
      ]
    },
    {
      "name": "offerVaultDepositEvent",
      "discriminator": [
        145,
        212,
        252,
        95,
        149,
        4,
        227,
        27
      ]
    },
    {
      "name": "offerVaultWithdrawEvent",
      "discriminator": [
        29,
        35,
        115,
        218,
        32,
        155,
        211,
        94
      ]
    },
    {
      "name": "offerVectorAddedEvent",
      "discriminator": [
        104,
        34,
        244,
        250,
        33,
        201,
        53,
        103
      ]
    },
    {
      "name": "offerVectorDeletedEvent",
      "discriminator": [
        11,
        85,
        222,
        27,
        101,
        98,
        160,
        167
      ]
    },
    {
      "name": "takeOfferEvent",
      "discriminator": [
        146,
        209,
        78,
        101,
        43,
        83,
        167,
        4
      ]
    },
    {
      "name": "takeOfferPermissionlessEvent",
      "discriminator": [
        77,
        13,
        9,
        252,
        35,
        255,
        82,
        134
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "mathOverflow",
      "msg": "Math overflow"
    }
  ],
  "types": [
    {
      "name": "adminState",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admins",
            "type": {
              "array": [
                "pubkey",
                20
              ]
            }
          }
        ]
      }
    },
    {
      "name": "bossUpdated",
      "docs": [
        "Event emitted when the boss is updated in the program state."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "oldBoss",
            "docs": [
              "The previous boss’s public key."
            ],
            "type": "pubkey"
          },
          {
            "name": "newBoss",
            "docs": [
              "The new boss’s public key."
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "closeOfferEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "boss",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "killSwitchState",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "isKilled",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "mintAuthorityTransferredToBossEvent",
      "docs": [
        "Event emitted when mint authority is successfully transferred from program PDA back to boss"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "The mint whose authority was transferred"
            ],
            "type": "pubkey"
          },
          {
            "name": "oldAuthority",
            "docs": [
              "The previous authority (program PDA)"
            ],
            "type": "pubkey"
          },
          {
            "name": "newAuthority",
            "docs": [
              "The new authority (boss)"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "mintAuthorityTransferredToProgramEvent",
      "docs": [
        "Event emitted when mint authority is successfully transferred from boss to program PDA"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "The mint whose authority was transferred"
            ],
            "type": "pubkey"
          },
          {
            "name": "oldAuthority",
            "docs": [
              "The previous authority (boss)"
            ],
            "type": "pubkey"
          },
          {
            "name": "newAuthority",
            "docs": [
              "The new authority (program PDA)"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offer",
      "docs": [
        "Offer struct for token exchange with dynamic pricing"
      ],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "tokenInMint",
            "type": "pubkey"
          },
          {
            "name": "tokenOutMint",
            "type": "pubkey"
          },
          {
            "name": "vectors",
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "offerVector"
                  }
                },
                10
              ]
            }
          },
          {
            "name": "counter",
            "type": "u64"
          },
          {
            "name": "feeBasisPoints",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "offerAccount",
      "docs": [
        "Account holding MAX_OFFERS Offer instances"
      ],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offers",
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "offer"
                  }
                },
                10
              ]
            }
          },
          {
            "name": "counter",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "offerFeeUpdatedEvent",
      "docs": [
        "Event emitted when a offer's fee is updated."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "oldFeeBasisPoints",
            "type": "u64"
          },
          {
            "name": "newFeeBasisPoints",
            "type": "u64"
          },
          {
            "name": "boss",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offerMadeEvent",
      "docs": [
        "Event emitted when an offer is created."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "feeBasisPoints",
            "type": "u64"
          },
          {
            "name": "boss",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offerVaultAuthority",
      "type": {
        "kind": "struct",
        "fields": []
      }
    },
    {
      "name": "offerVaultDepositEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "boss",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offerVaultWithdrawEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "boss",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offerVector",
      "docs": [
        "Time vector for offers with pricing information"
      ],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "vectorId",
            "type": "u64"
          },
          {
            "name": "startTime",
            "type": "u64"
          },
          {
            "name": "baseTime",
            "type": "u64"
          },
          {
            "name": "basePrice",
            "type": "u64"
          },
          {
            "name": "apr",
            "docs": [
              "Annual Percentage Rate (APR)",
              "",
              "APR represents the annualized rate of return for this offer.",
              "It is scaled by 1,000,000 for precision (6 decimal places).",
              "",
              "Examples:",
              "- 0 = 0% APR (fixed price, no yield over time)",
              "- 36_500 = 0.0365% APR (3.65% annual rate)",
              "- 1_000_000 = 1% APR",
              "- 10_000_000 = 10% APR",
              "",
              "The APR determines how the price increases over time intervals."
            ],
            "type": "u64"
          },
          {
            "name": "priceFixDuration",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "offerVectorAddedEvent",
      "docs": [
        "Event emitted when a time vector is added to an offer."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "vectorId",
            "type": "u64"
          },
          {
            "name": "startTime",
            "type": "u64"
          },
          {
            "name": "baseTime",
            "type": "u64"
          },
          {
            "name": "basePrice",
            "type": "u64"
          },
          {
            "name": "apr",
            "type": "u64"
          },
          {
            "name": "priceFixDuration",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "offerVectorDeletedEvent",
      "docs": [
        "Event emitted when a time vector is deleted from a offer."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "vectorId",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "permissionlessAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "name",
            "type": "string"
          }
        ]
      }
    },
    {
      "name": "state",
      "docs": [
        "Represents the program state in the Onre App program.",
        "",
        "Stores the current boss's public key, used for authorization across instructions.",
        "",
        "# Fields",
        "- `boss`: Public key of the current boss, set via `initialize` and updated via `set_boss`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "boss",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "takeOfferEvent",
      "docs": [
        "Event emitted when an offer is successfully taken"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "docs": [
              "The ID of the offer that was taken"
            ],
            "type": "u64"
          },
          {
            "name": "tokenInAmount",
            "docs": [
              "Amount of token_in paid by the user (excluding fee)"
            ],
            "type": "u64"
          },
          {
            "name": "tokenOutAmount",
            "docs": [
              "Amount of token_out received by the user"
            ],
            "type": "u64"
          },
          {
            "name": "feeAmount",
            "docs": [
              "Fee amount paid by the user in token_in"
            ],
            "type": "u64"
          },
          {
            "name": "user",
            "docs": [
              "Public key of the user who took the offer"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "takeOfferPermissionlessEvent",
      "docs": [
        "Event emitted when a offer is successfully taken via permissionless flow"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "docs": [
              "The ID of the offer that was taken"
            ],
            "type": "u64"
          },
          {
            "name": "tokenInAmount",
            "docs": [
              "Amount of token_in paid by the user"
            ],
            "type": "u64"
          },
          {
            "name": "tokenOutAmount",
            "docs": [
              "Amount of token_out received by the user"
            ],
            "type": "u64"
          },
          {
            "name": "feeAmount",
            "docs": [
              "Fee amount paid by the user in token_in"
            ],
            "type": "u64"
          },
          {
            "name": "user",
            "docs": [
              "Public key of the user who took the offer"
            ],
            "type": "pubkey"
          }
        ]
      }
    }
  ]
};
