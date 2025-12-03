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
    "- Making offers with dynamic pricing (`make_offer`).",
    "- Taking offers with current market pricing (`take_offer`, `take_offer_permissionless`).",
    "- Managing offer vectors for price control (`add_offer_vector`, `delete_offer_vector`).",
    "- Closing offers (`close_offer`).",
    "- Program state initialization and management (`initialize`, `set_boss`, `add_admin`, `remove_admin`).",
    "- Vault operations for token deposits and withdrawals (`offer_vault_deposit`, `offer_vault_withdraw`).",
    "- Market information queries (`get_nav`, `get_apy`, `get_tvl`, `get_circulating_supply`).",
    "- Mint authority management (`transfer_mint_authority_to_program`, `transfer_mint_authority_to_boss`).",
    "- Emergency controls (`set_kill_switch`) and approval mechanisms (`set_approver`).",
    "",
    "# Dynamic Pricing Model",
    "The price for offers is determined by time-based vectors with APR (Annual Percentage Rate) growth:",
    "- `base_time`: The timestamp when the vector becomes active.",
    "- `base_price`: The initial price at the base_time with 9 decimal precision.",
    "- `apr`: Annual percentage rate scaled by 1,000,000 (e.g., 1_000_000 = 1% APR).",
    "- `price_fix_duration`: Duration in seconds for each discrete pricing step.",
    "The price increases over time based on the APR, calculated in discrete intervals.",
    "",
    "# Security",
    "- Access controls are enforced, for example, ensuring only the `boss` can create offers or update critical state.",
    "- PDA (Program Derived Address) accounts are used for offer and token authorities, ensuring ownership.",
    "- Events are emitted for significant actions (e.g., `OfferMadeOne`, `OfferTakenTwo`) for off-chain traceability."
  ],
  "instructions": [
    {
      "name": "acceptBoss",
      "docs": [
        "Accepts the proposed boss transfer.",
        "",
        "Delegates to `accept_boss::accept_boss` to complete the ownership transfer.",
        "This is the second step in a two-step ownership transfer process.",
        "Only the proposed boss can call this instruction to accept and become the new boss.",
        "Emits a `BossAcceptedEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `AcceptBoss`."
      ],
      "discriminator": [
        152,
        63,
        117,
        209,
        67,
        11,
        250,
        242
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "Program state account containing the boss and proposed_boss",
            "",
            "Must be mutable to allow boss field modification."
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
          "name": "newBoss",
          "docs": [
            "The proposed new boss account accepting the ownership transfer"
          ],
          "signer": true
        }
      ],
      "args": []
    },
    {
      "name": "addAdmin",
      "docs": [
        "Adds a new admin to the state.",
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
          "name": "state",
          "docs": [
            "Program state account containing the admin list",
            "",
            "Must be mutable to allow admin list modifications and have the",
            "boss account as the authorized signer for admin management."
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
            "The boss account authorized to add new admins"
          ],
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
      "name": "addApprover",
      "docs": [
        "Adds a trusted authority for approval verification.",
        "",
        "This instruction allows the boss to add an approver to one of the two available",
        "approver slots. If both slots are already filled, the instruction will fail.",
        "",
        "# Arguments",
        "- `ctx`: Context for `AddApprover`.",
        "- `approver`: Public key of the approver to add."
      ],
      "discriminator": [
        213,
        245,
        135,
        79,
        129,
        129,
        22,
        80
      ],
      "accounts": [
        {
          "name": "state",
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
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "approver",
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
        "Creates a new time vector with auto-generated vector_start_timestamp for the specified offer.",
        "Emits a `OfferVectorAdded` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `AddOfferVector`.",
        "- `start_time`: Unix timestamp when the vector becomes active.",
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
          "name": "offer",
          "docs": [
            "The offer account to which the pricing vector will be added",
            "",
            "This account is validated as a PDA derived from token mint addresses",
            "and contains the array of pricing vectors for the offer."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The input token mint account for offer validation"
          ]
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "The output token mint account for offer validation"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing boss authorization"
          ],
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
            "The boss account authorized to add pricing vectors to offers"
          ],
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "startTime",
          "type": {
            "option": "u64"
          }
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
      "name": "cancelRedemptionRequest",
      "docs": [
        "Cancels a redemption request.",
        "",
        "Delegates to `redemption::cancel_redemption_request`.",
        "This instruction cancels a pending redemption request. The request can be cancelled",
        "by the redeemer, redemption_admin, or boss. Upon cancellation, the status is changed",
        "to cancelled and the amount is subtracted from the redemption offer's requested_redemptions.",
        "The redemption request account is NOT closed.",
        "Emits a `RedemptionRequestCancelledEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `CancelRedemptionRequest`.",
        "",
        "# Access Control",
        "- Signer must be one of: redeemer, redemption_admin, or boss",
        "- Request must be in pending state (status = 0)"
      ],
      "discriminator": [
        77,
        155,
        4,
        179,
        114,
        233,
        162,
        45
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "Program state account containing redemption_admin and boss for authorization"
          ],
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
          "name": "redemptionOffer",
          "docs": [
            "The redemption offer account"
          ],
          "writable": true
        },
        {
          "name": "redemptionRequest",
          "docs": [
            "The redemption request account to cancel"
          ],
          "writable": true
        },
        {
          "name": "signer",
          "docs": [
            "The signer who is cancelling the request",
            "Can be either the redeemer, redemption_admin, or boss"
          ],
          "signer": true
        },
        {
          "name": "redemptionVaultAuthority",
          "docs": [
            "Program-derived authority that controls redemption vault token accounts",
            "",
            "This PDA manages the redemption vault token accounts and enables the program",
            "to return locked tokens when requests are cancelled."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  100,
                  101,
                  109,
                  112,
                  116,
                  105,
                  111,
                  110,
                  95,
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
            "The token mint for token_in (input token)"
          ]
        },
        {
          "name": "vaultTokenAccount",
          "docs": [
            "Redemption vault's token account serving as the source of locked tokens",
            "",
            "Contains the tokens that were locked when the request was created."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "redemptionVaultAuthority"
              },
              {
                "kind": "account",
                "path": "tokenProgram"
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
          "name": "redeemerTokenAccount",
          "docs": [
            "Redeemer's token account serving as the destination for returned tokens",
            "",
            "Receives back the tokens that were locked in the redemption request."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "redemptionRequest"
              },
              {
                "kind": "account",
                "path": "tokenProgram"
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
          "name": "tokenProgram",
          "docs": [
            "Token program interface for transfer operations"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "clearAdmins",
      "docs": [
        "Clears all admins from the state.",
        "",
        "Delegates to `admin::clear_admins` to remove all admins from the admin list.",
        "Only the boss can call this instruction to clear all admins."
      ],
      "discriminator": [
        39,
        200,
        132,
        30,
        196,
        160,
        73,
        55
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "Program state account containing the admin list to be cleared",
            "",
            "Must be mutable to allow admin list modifications and have the",
            "boss account as the authorized signer for admin management."
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
            "The boss account authorized to clear all admin privileges"
          ],
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": []
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
        "- `ctx`: Context for `CloseOffer`."
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
          "name": "offer",
          "docs": [
            "The offer account to be closed and its rent reclaimed",
            "",
            "This account is validated as a PDA derived from token mint addresses.",
            "The account will be closed and its rent transferred to the boss."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The input token mint account for offer validation"
          ]
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "The output token mint account for offer validation"
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The boss account authorized to close offers and receive rent"
          ],
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing boss authorization"
          ],
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
          "name": "systemProgram",
          "docs": [
            "System program required for account closure and rent transfer"
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "closeState",
      "docs": [
        "Closes the program state account and returns the rent to the boss.",
        "",
        "Delegates to `state_operations::close_state`.",
        "This instruction permanently deletes the program's main state account",
        "and transfers its rent balance back to the boss. Once closed, the state",
        "cannot be recovered and the program becomes effectively non-functional.",
        "Only the boss can call this instruction.",
        "Emits a `StateClosedEvent` upon success.",
        "",
        "# Warning",
        "This is a destructive operation that effectively disables the program.",
        "Use with extreme caution.",
        "",
        "# Arguments",
        "- `ctx`: Context for `CloseState`."
      ],
      "discriminator": [
        25,
        1,
        184,
        101,
        200,
        245,
        210,
        246
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "The state account to be closed and its rent reclaimed",
            "",
            "This account is validated as a PDA derived from the \"state\" seed.",
            "The account will be closed and its rent transferred to the boss.",
            ""
          ],
          "writable": true
        },
        {
          "name": "boss",
          "docs": [
            "The boss account authorized to close the state and receive rent",
            "",
            "Must match the boss stored in the state account.",
            "This signer will receive the rent from the closed state account."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program required for account closure and rent transfer"
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "configureMaxSupply",
      "docs": [
        "Configures the maximum supply cap for ONyc token minting.",
        "",
        "Delegates to `state_operations::configure_max_supply`.",
        "This instruction allows the boss to set or update the maximum supply cap",
        "that restricts ONyc token minting. Setting to 0 removes the cap.",
        "Emits a `MaxSupplyConfigured` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `ConfigureMaxSupply`.",
        "- `max_supply`: The maximum supply cap in base units (0 = no cap)."
      ],
      "discriminator": [
        145,
        100,
        133,
        229,
        142,
        59,
        96,
        62
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "Program state account containing the max supply configuration",
            "",
            "Must be mutable to allow max supply updates and have the boss account",
            "as the authorized signer for supply cap management."
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
            "The boss account authorized to configure the max supply"
          ],
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "maxSupply",
          "type": "u64"
        }
      ]
    },
    {
      "name": "createRedemptionRequest",
      "docs": [
        "Creates a redemption request.",
        "",
        "Delegates to `redemption::create_redemption_request`.",
        "This instruction creates a new redemption request that allows users to request",
        "redemption of token_in tokens for token_out tokens at a future time. The request must",
        "be authorized by the redemption admin and uses a nonce to prevent replay attacks.",
        "Emits a `RedemptionRequestCreatedEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `CreateRedemptionRequest`.",
        "- `amount`: Amount of token_in tokens to redeem.",
        "- `expires_at`: Unix timestamp when the request expires.",
        "- `nonce`: User's nonce for replay attack prevention (must match UserNonceAccount)."
      ],
      "discriminator": [
        201,
        53,
        181,
        254,
        115,
        137,
        70,
        151
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "Program state account containing redemption_admin authorization"
          ],
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
          "name": "redemptionOffer",
          "docs": [
            "The redemption offer account"
          ],
          "writable": true
        },
        {
          "name": "redemptionRequest",
          "docs": [
            "The redemption request account"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  100,
                  101,
                  109,
                  112,
                  116,
                  105,
                  111,
                  110,
                  95,
                  114,
                  101,
                  113,
                  117,
                  101,
                  115,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "redemptionOffer"
              },
              {
                "kind": "account",
                "path": "redeemer"
              },
              {
                "kind": "arg",
                "path": "nonce"
              }
            ]
          }
        },
        {
          "name": "userNonceAccount",
          "docs": [
            "User nonce account for preventing replay attacks",
            "",
            "This account tracks the user's current nonce to ensure each request is unique."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  110,
                  111,
                  110,
                  99,
                  101,
                  95,
                  97,
                  99,
                  99,
                  111,
                  117,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "redeemer"
              }
            ]
          }
        },
        {
          "name": "redeemer",
          "docs": [
            "User requesting the redemption (pays for account creation)"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "redemptionAdmin",
          "docs": [
            "Redemption admin must sign to authorize the request"
          ],
          "signer": true
        },
        {
          "name": "redemptionVaultAuthority",
          "docs": [
            "Program-derived authority that controls redemption vault token accounts",
            "",
            "This PDA manages the redemption vault token accounts and enables the program",
            "to hold tokens until redemption requests are fulfilled or cancelled."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  100,
                  101,
                  109,
                  112,
                  116,
                  105,
                  111,
                  110,
                  95,
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
            "The token mint for token_in (input token)"
          ]
        },
        {
          "name": "redeemerTokenAccount",
          "docs": [
            "Redeemer's token account serving as the source of deposited tokens",
            "",
            "Must have sufficient balance to cover the requested amount."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "redeemer"
              },
              {
                "kind": "account",
                "path": "tokenProgram"
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
          "name": "vaultTokenAccount",
          "docs": [
            "Redemption vault's token account serving as the destination for locked tokens",
            "",
            "Must exist. Stores tokens that are locked until the redemption request is",
            "fulfilled or cancelled."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "redemptionVaultAuthority"
              },
              {
                "kind": "account",
                "path": "tokenProgram"
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
          "name": "tokenProgram",
          "docs": [
            "Token program interface for transfer operations"
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
            "System program for account creation"
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "expiresAt",
          "type": "u64"
        },
        {
          "name": "nonce",
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
        "- `vector_start_time`: Start time of the vector to delete."
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
          "name": "offer",
          "docs": [
            "The offer account from which the pricing vector will be deleted",
            "",
            "This account is validated as a PDA derived from token mint addresses",
            "and contains the array of pricing vectors for the offer."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The input token mint account for offer validation"
          ]
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "The output token mint account for offer validation"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing boss authorization"
          ],
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
            "The boss account authorized to delete pricing vectors from offers"
          ],
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "vectorStartTime",
          "type": "u64"
        }
      ]
    },
    {
      "name": "getApy",
      "docs": [
        "Gets the current APY (Annual Percentage Yield) for a specific offer.",
        "",
        "Delegates to `market_info::get_apy`.",
        "This is a read-only instruction that calculates and returns the current APY",
        "by converting the stored APR using daily compounding formula.",
        "Emits a `GetAPYEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `GetAPY`.",
        "",
        "# Returns",
        "- `Ok(apy)`: The calculated APY scaled by 1_000_000 (returns the mantissa, with scale=6)"
      ],
      "discriminator": [
        194,
        123,
        183,
        54,
        181,
        74,
        194,
        97
      ],
      "accounts": [
        {
          "name": "offer",
          "docs": [
            "The offer account containing the pricing vectors and APR data",
            "",
            "This account is validated as a PDA derived from the \"offer\" seed combined",
            "with both token mint addresses. Contains the time-based pricing vectors",
            "that include the APR values used for APY calculation."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The input token mint account for offer validation",
            "",
            "Must match the token_in_mint stored in the offer account to ensure",
            "the correct offer is being queried. This validation prevents",
            "accidental queries against incorrect token pairs."
          ]
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "The output token mint account for offer validation",
            "",
            "Must match the token_out_mint stored in the offer account to ensure",
            "the correct offer is being queried. This validation prevents",
            "accidental queries against incorrect token pairs."
          ]
        }
      ],
      "args": [],
      "returns": "u64"
    },
    {
      "name": "getCirculatingSupply",
      "docs": [
        "Delegates to `market_info::get_circulating_supply`.",
        "This is a read-only instruction that calculates and returns the current circulating supply",
        "for an offer based on the total token supply minus the vault amount.",
        "circulating_supply = total_supply - vault_amount",
        "Emits a `GetCirculatingSupplyEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `GetCirculatingSupply`.",
        "",
        "# Returns",
        "- `Ok(circulating_supply)`: The calculated circulating supply for the offer in base units"
      ],
      "discriminator": [
        132,
        168,
        96,
        104,
        217,
        255,
        111,
        152
      ],
      "accounts": [
        {
          "name": "onycMint",
          "docs": [
            "The ONyc token mint containing total supply information"
          ],
          "relations": [
            "state"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing the ONyc mint reference"
          ],
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
          "name": "vaultAuthority",
          "docs": [
            "The vault authority PDA that controls vault token accounts"
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
          "name": "onycVaultAccount",
          "docs": [
            "The vault's ONyc token account to exclude from circulating supply",
            "",
            "This account holds tokens that are not considered in circulation.",
            "The account address is validated to match the expected ATA address",
            "and can be uninitialized (treated as zero balance)."
          ]
        },
        {
          "name": "tokenProgram",
          "docs": [
            "SPL Token program for account validation"
          ]
        }
      ],
      "args": [],
      "returns": "u64"
    },
    {
      "name": "getNav",
      "docs": [
        "Gets the current NAV (price) for a specific offer.",
        "",
        "Delegates to `market_info::get_nav`.",
        "This is a read-only instruction that calculates and returns the current price",
        "for an offer based on its time vectors and APR parameters.",
        "Emits a `GetNAVEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `GetNAV`.",
        "",
        "# Returns",
        "- `Ok(current_price)`: The calculated current price (mantissa) for the offer with scale=9"
      ],
      "discriminator": [
        200,
        89,
        76,
        53,
        215,
        218,
        63,
        21
      ],
      "accounts": [
        {
          "name": "offer",
          "docs": [
            "The offer account containing pricing vectors and configuration",
            "",
            "This account is validated as a PDA derived from token mint addresses",
            "and contains time-based pricing vectors for price calculation."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The input token mint account for offer validation"
          ]
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "The output token mint account for offer validation"
          ]
        }
      ],
      "args": [],
      "returns": "u64"
    },
    {
      "name": "getNavAdjustment",
      "docs": [
        "Gets the NAV adjustment (price change) for a specific offer.",
        "",
        "Delegates to `market_info::get_nav_adjustment`.",
        "This is a read-only instruction that calculates the price difference",
        "between the current vector and the previous vector at the current time.",
        "Returns a signed integer representing the price change.",
        "Emits a `GetNavAdjustmentEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `GetNavAdjustment`.",
        "",
        "# Returns",
        "- `Ok(adjustment)`: The calculated price adjustment (current - previous) as a signed integer,",
        "returns the mantissa with scale=9"
      ],
      "discriminator": [
        70,
        198,
        229,
        129,
        238,
        233,
        143,
        94
      ],
      "accounts": [
        {
          "name": "offer",
          "docs": [
            "The offer account containing pricing vectors for adjustment calculation",
            "",
            "This account is validated as a PDA derived from token mint addresses",
            "and contains multiple time-based pricing vectors for comparison."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The input token mint account for offer validation"
          ]
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "The output token mint account for offer validation"
          ]
        }
      ],
      "args": [],
      "returns": "i64"
    },
    {
      "name": "getTvl",
      "docs": [
        "Gets the current TVL (Total Value Locked) for a specific offer with 9 decimal precision",
        "",
        "Delegates to `market_info::get_tvl`.",
        "This is a read-only instruction that calculates and returns the current TVL",
        "for an offer based on the token_out supply and current NAV (price).",
        "TVL = token_out_supply * current_NAV",
        "Emits a `GetTVLEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `GetTVL`.",
        "",
        "# Returns",
        "- `Ok(tvl)`: The calculated TVL (mantissa) for the offer with scale=9"
      ],
      "discriminator": [
        88,
        225,
        219,
        204,
        86,
        91,
        184,
        51
      ],
      "accounts": [
        {
          "name": "offer",
          "docs": [
            "The offer account containing pricing vectors for current price calculation",
            "",
            "This account is validated as a PDA derived from token mint addresses",
            "and contains time-based pricing vectors for TVL calculation."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The input token mint account for offer validation"
          ]
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "The output token mint account containing total supply information"
          ]
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "The vault authority PDA that controls vault token accounts"
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
          "name": "vaultTokenOutAccount",
          "docs": [
            "The vault's token_out account to exclude from circulating supply",
            "",
            "This account holds tokens that should not be included in TVL calculations.",
            "The account address is validated to match the expected ATA address",
            "and can be uninitialized (treated as zero balance)."
          ]
        },
        {
          "name": "tokenOutProgram",
          "docs": [
            "SPL Token program for vault account validation"
          ]
        }
      ],
      "args": [],
      "returns": "u64"
    },
    {
      "name": "initialize",
      "docs": [
        "Initializes the program state and authority accounts.",
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
            "The program state account to be created and initialized",
            "",
            "This account stores all global program configuration including:",
            "- Boss public key (program authority)",
            "- Kill switch state (initially disabled)",
            "- ONyc mint reference",
            "- Admin list (initially empty)",
            "- Approver for signature verification (initially unset)",
            "",
            "The account is created as a PDA derived from the \"state\" seed to ensure",
            "deterministic addressing and program ownership."
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
          "name": "mintAuthority",
          "docs": [
            "The offer mint authority account to initialize, rent paid by `boss`."
          ],
          "writable": true,
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
          "name": "offerVaultAuthority",
          "docs": [
            "The offer vault authority account to initialize, rent paid by `boss`."
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
          "name": "boss",
          "docs": [
            "The initial boss who will have full authority over the program",
            "",
            "This signer becomes the program's boss and gains the ability to:",
            "- Create and manage offers",
            "- Update program state (boss, admins, kill switch, approver)",
            "- Perform administrative operations",
            "- Pay for the state account creation"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "program",
          "address": "onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe"
        },
        {
          "name": "programData",
          "docs": [
            "We'll verify its address in code."
          ],
          "optional": true
        },
        {
          "name": "onycMint",
          "docs": [
            "The ONyc token mint that this program will manage",
            "",
            "This mint represents the protocol's native token and is used for:",
            "- Token minting operations when program has mint authority",
            "- Reference in various program calculations and operations"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program required for account creation and rent payment"
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "initializePermissionlessAuthority",
      "docs": [
        "Initializes a permissionless account.",
        "",
        "Delegates to `initialize::initialize_permissionless_authority` to create a new permissionless account.",
        "The account is created as a PDA with the seed \"permissionless-1\".",
        "Only the boss can initialize permissionless accounts."
      ],
      "discriminator": [
        89,
        93,
        43,
        180,
        148,
        16,
        238,
        24
      ],
      "accounts": [
        {
          "name": "permissionlessAuthority",
          "docs": [
            "The permissionless account to be created.",
            "",
            "# Note",
            "- Space is allocated as `8 + PermissionlessAuthority::INIT_SPACE` bytes",
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
          ],
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
          "name": "vaultAuthority",
          "docs": [
            "Program-derived authority that controls offer vault token accounts",
            "",
            "This PDA manages token transfers and burning operations when the program",
            "has mint authority for efficient burn/mint architecture."
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
            "The input token mint for the offer"
          ]
        },
        {
          "name": "tokenInProgram",
          "docs": [
            "Token program interface for the input token"
          ]
        },
        {
          "name": "vaultTokenInAccount",
          "docs": [
            "Vault account for storing input tokens during burn/mint operations",
            "",
            "Created automatically if needed. Used for temporary token storage",
            "when the program has mint authority and needs to burn tokens."
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
            "The output token mint for the offer"
          ]
        },
        {
          "name": "offer",
          "docs": [
            "The offer account storing exchange configuration and pricing vectors",
            "",
            "This account is derived from token mint addresses ensuring unique",
            "offers per token pair. Contains fee settings, approval requirements,",
            "and pricing vector array for dynamic pricing."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing boss authorization"
          ],
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
            "The boss account authorized to create offers and pay for account creation"
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
            "System program for account creation and rent payment"
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "feeBasisPoints",
          "type": "u16"
        },
        {
          "name": "needsApproval",
          "type": "bool"
        },
        {
          "name": "allowPermissionless",
          "type": "bool"
        }
      ]
    },
    {
      "name": "makeRedemptionOffer",
      "docs": [
        "Creates a redemption offer for converting output tokens from standard offers back",
        "to input tokens.",
        "",
        "Delegates to `redemption::make_redemption_offer`.",
        "This instruction initializes a new redemption offer that allows users to redeem",
        "token_out tokens from standard Offer (e.g. ONyc) for token_in tokens (e.g., USDC) at",
        "the current NAV price. The redemption offer is the inverse of the standard Offer.",
        "",
        "The redemption offer PDA is derived with reversed token order compared to the",
        "original offer, reflecting the inverse nature of the redemption operation.",
        "Emits a `RedemptionOfferCreatedEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `MakeRedemptionOffer`.",
        "",
        "# Access Control",
        "- Only the boss or redemption_admin can call this instruction"
      ],
      "discriminator": [
        6,
        130,
        180,
        160,
        163,
        166,
        51,
        41
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "Program state account containing boss and redemption_admin authorization"
          ],
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
          "name": "offer",
          "docs": [
            "The original offer that this redemption offer is associated with",
            "",
            "The redemption offer uses the inverse token pair of the original offer.",
            "The offer must be derived from redemption offer token_out_mint (token_in in original offer)",
            "and token_in_mint (token_out in original offer)."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              }
            ]
          }
        },
        {
          "name": "redemptionVaultAuthority",
          "docs": [
            "Program-derived authority that controls redemption offer vault token accounts",
            "",
            "This PDA manages token transfers for redemption operations."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  100,
                  101,
                  109,
                  112,
                  116,
                  105,
                  111,
                  110,
                  95,
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
            "The input token mint for redemptions",
            "",
            "This is the token_out_mint from the original offer."
          ]
        },
        {
          "name": "tokenInProgram",
          "docs": [
            "Token program interface for the input token"
          ]
        },
        {
          "name": "vaultTokenInAccount",
          "docs": [
            "Vault account for storing input tokens during redemption operations",
            "",
            "Created automatically if needed. Used for holding ONyc tokens before burning."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "redemptionVaultAuthority"
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
            "The output token mint for redemptions (e.g., USDC)",
            "",
            "This is the token_in_mint from the original offer."
          ]
        },
        {
          "name": "tokenOutProgram",
          "docs": [
            "Token program interface for the output token"
          ]
        },
        {
          "name": "vaultTokenOutAccount",
          "docs": [
            "Vault account for storing output tokens (e.g., USDC) for redemption payouts",
            "",
            "Created automatically if needed. Used for distributing stable tokens to redeemers."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "redemptionVaultAuthority"
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
          "name": "redemptionOffer",
          "docs": [
            "The redemption offer account storing redemption configuration",
            "",
            "This account is derived from token mint addresses in the same order as Offer",
            "but with the tokens reversed (token_in from Offer becomes token_out here)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  100,
                  101,
                  109,
                  112,
                  116,
                  105,
                  111,
                  110,
                  95,
                  111,
                  102,
                  102,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "signer",
          "docs": [
            "The account creating the redemption offer (must be boss or redemption_admin)"
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
            "System program for account creation and rent payment"
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "mintTo",
      "docs": [
        "Mints ONyc tokens to the boss's account.",
        "",
        "Delegates to `state_operations::mint_to` to mint ONyc tokens.",
        "Only the boss can call this instruction to mint ONyc tokens to their account.",
        "The program must have mint authority for the ONyc token.",
        "Emits a `OnycTokensMinted` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `MintTo`.",
        "- `amount`: Amount of ONyc tokens to mint."
      ],
      "discriminator": [
        241,
        34,
        48,
        186,
        37,
        179,
        123,
        192
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "The program state account containing boss and ONyc mint validation"
          ],
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
            "The boss authorized to perform minting operations",
            "",
            "Must be the boss stored in the program state and pay for any",
            "account creation if the token account doesn't exist."
          ],
          "writable": true,
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "onycMint",
          "docs": [
            "The ONyc token mint account for minting new tokens",
            "",
            "Must match the ONyc mint stored in program state and be mutable",
            "to allow supply updates during minting."
          ],
          "writable": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "bossOnycAccount",
          "docs": [
            "The boss's ONyc token account to receive minted tokens",
            "",
            "If the account doesn't exist, it will be created automatically",
            "as an Associated Token Account with the boss as the authority."
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
                "path": "onycMint"
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
          "name": "mintAuthority",
          "docs": [
            "Program-derived account that serves as the mint authority",
            "",
            "This PDA must be the current mint authority for the ONyc token.",
            "Validated to ensure the program has permission to mint tokens."
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
            "SPL Token program for minting operations"
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
            "System program required for account creation and rent payment"
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
            "Program-derived authority that controls vault token accounts",
            "",
            "This PDA manages the vault token accounts and enables the program",
            "to distribute tokens during offer executions."
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
            "The token mint for the deposit operation"
          ]
        },
        {
          "name": "bossTokenAccount",
          "docs": [
            "Boss's token account serving as the source of deposited tokens",
            "",
            "Must have sufficient balance to cover the requested deposit amount."
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
            "Vault's token account serving as the destination for deposited tokens",
            "",
            "Created automatically if it doesn't exist. Stores tokens that can be",
            "distributed during offer executions when minting is not available."
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
            "The boss account authorized to deposit tokens and pay for account creation"
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
            "Program state account containing boss authorization"
          ],
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
          "name": "tokenProgram",
          "docs": [
            "Token program interface for transfer operations"
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
            "System program for account creation and rent payment"
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
            "Program-derived authority that controls vault token accounts",
            "",
            "This PDA manages the vault token accounts and signs the withdrawal",
            "transfer using program-derived signatures."
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
            "The token mint for the withdrawal operation"
          ]
        },
        {
          "name": "bossTokenAccount",
          "docs": [
            "Boss's token account serving as the destination for withdrawn tokens",
            "",
            "Created automatically if it doesn't exist. Receives tokens withdrawn",
            "from the vault for boss fund management."
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
            "Vault's token account serving as the source of withdrawn tokens",
            "",
            "Must have sufficient balance to cover the requested withdrawal amount.",
            "Controlled by the vault authority PDA."
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
            "The boss account authorized to withdraw tokens and pay for account creation"
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
            "Program state account containing boss authorization"
          ],
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
          "name": "tokenProgram",
          "docs": [
            "Token program interface for transfer operations"
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
            "System program for account creation and rent payment"
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
      "name": "proposeBoss",
      "docs": [
        "Proposes a new boss for ownership transfer.",
        "",
        "Delegates to `propose_boss::propose_boss` to propose a new boss authority.",
        "This is the first step in a two-step ownership transfer process.",
        "The proposed boss must then call accept_boss to complete the transfer.",
        "Emits a `BossProposedEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `ProposeBoss`.",
        "- `new_boss`: Public key of the proposed new boss."
      ],
      "discriminator": [
        163,
        199,
        158,
        47,
        155,
        78,
        174,
        173
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "Program state account containing the boss authority",
            "",
            "Must be mutable to allow proposed_boss field modification and have the current",
            "boss account as the authorized signer."
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
            "The current boss account proposing the ownership transfer"
          ],
          "signer": true,
          "relations": [
            "state"
          ]
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
      "name": "redemptionVaultDeposit",
      "docs": [
        "Deposits tokens into the redemption vault.",
        "",
        "Delegates to `vault_operations::redemption_vault_deposit`.",
        "Transfers tokens from boss's account to redemption vault's token account for the specified mint.",
        "Creates vault token account if it doesn't exist using init_if_needed.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `RedemptionVaultDeposit`.",
        "- `amount`: Amount of tokens to deposit."
      ],
      "discriminator": [
        67,
        104,
        107,
        131,
        141,
        2,
        140,
        122
      ],
      "accounts": [
        {
          "name": "redemptionVaultAuthority",
          "docs": [
            "Program-derived authority that controls redemption vault token accounts",
            "",
            "This PDA manages the redemption vault token accounts and enables the program",
            "to distribute tokens during redemption executions."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  100,
                  101,
                  109,
                  112,
                  116,
                  105,
                  111,
                  110,
                  95,
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
            "The token mint for the deposit operation"
          ]
        },
        {
          "name": "bossTokenAccount",
          "docs": [
            "Boss's token account serving as the source of deposited tokens",
            "",
            "Must have sufficient balance to cover the requested deposit amount."
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
            "Redemption vault's token account serving as the destination for deposited tokens",
            "",
            "Created automatically if it doesn't exist. Stores tokens that can be",
            "distributed during redemption executions when minting is not available."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "redemptionVaultAuthority"
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
            "The boss account authorized to deposit tokens and pay for account creation"
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
            "Program state account containing boss authorization"
          ],
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
          "name": "tokenProgram",
          "docs": [
            "Token program interface for transfer operations"
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
            "System program for account creation and rent payment"
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
      "name": "redemptionVaultWithdraw",
      "docs": [
        "Withdraws tokens from the redemption vault.",
        "",
        "Delegates to `vault_operations::redemption_vault_withdraw`.",
        "Transfers tokens from redemption vault's token account to boss's account for the specified mint.",
        "Creates boss token account if it doesn't exist using init_if_needed.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `RedemptionVaultWithdraw`.",
        "- `amount`: Amount of tokens to withdraw."
      ],
      "discriminator": [
        48,
        214,
        145,
        15,
        168,
        122,
        39,
        48
      ],
      "accounts": [
        {
          "name": "redemptionVaultAuthority",
          "docs": [
            "Program-derived authority that controls redemption vault token accounts",
            "",
            "This PDA manages the redemption vault token accounts and signs the withdrawal",
            "transfer using program-derived signatures."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  100,
                  101,
                  109,
                  112,
                  116,
                  105,
                  111,
                  110,
                  95,
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
            "The token mint for the withdrawal operation"
          ]
        },
        {
          "name": "bossTokenAccount",
          "docs": [
            "Boss's token account serving as the destination for withdrawn tokens",
            "",
            "Created automatically if it doesn't exist. Receives tokens withdrawn",
            "from the redemption vault for boss fund management."
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
            "Redemption vault's token account serving as the source of withdrawn tokens",
            "",
            "Must have sufficient balance to cover the requested withdrawal amount.",
            "Controlled by the redemption vault authority PDA."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "redemptionVaultAuthority"
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
            "The boss account authorized to withdraw tokens and pay for account creation"
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
            "Program state account containing boss authorization"
          ],
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
          "name": "tokenProgram",
          "docs": [
            "Token program interface for transfer operations"
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
            "System program for account creation and rent payment"
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
        "Removes an admin from the state.",
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
          "name": "state",
          "docs": [
            "Program state account containing the admin list to be modified",
            "",
            "Must be mutable to allow admin list modifications and have the",
            "boss account as the authorized signer for admin management."
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
            "The boss account authorized to remove admin privileges"
          ],
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
      "name": "removeApprover",
      "docs": [
        "Removes a trusted authority from approval verification.",
        "",
        "This instruction allows the boss to remove an approver by their public key.",
        "The approver must exist in either approver1 or approver2 slot, otherwise",
        "the instruction will fail.",
        "",
        "# Arguments",
        "- `ctx`: Context for `RemoveApprover`.",
        "- `approver`: Public key of the approver to remove."
      ],
      "discriminator": [
        214,
        72,
        133,
        48,
        50,
        58,
        227,
        224
      ],
      "accounts": [
        {
          "name": "state",
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
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "approver",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "setKillSwitch",
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
        228,
        119,
        172,
        135,
        209,
        250,
        172,
        216
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "Program state account containing the kill switch flag",
            "",
            "Must be mutable to allow kill switch state modifications.",
            "The kill switch prevents offer operations when enabled."
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
          "name": "signer",
          "docs": [
            "The account attempting to modify the kill switch (boss or admin)"
          ],
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
      "name": "setOnycMint",
      "docs": [
        "Sets the Onyc mint in the state.",
        "",
        "Delegates to `state_operations::set_onyc_mint` to change the Onyc mint.",
        "Only the boss can call this instruction to set the Onyc mint.",
        "Emits a `OnycMintSetEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `SetOnycMint`."
      ],
      "discriminator": [
        177,
        83,
        119,
        179,
        44,
        141,
        201,
        24
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "Program state account containing the ONyc mint configuration",
            "",
            "Must be mutable to allow ONyc mint updates and have the boss account",
            "as the authorized signer for mint configuration management."
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
            "The boss account authorized to configure the ONyc mint"
          ],
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "onycMint",
          "docs": [
            "The ONyc token mint account to be set in program state"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "setRedemptionAdmin",
      "docs": [
        "Sets the redemption admin in the state.",
        "",
        "Delegates to `state_operations::set_redemption_admin` to change the redemption admin.",
        "Only the boss can call this instruction to set the redemption admin.",
        "Emits a `RedemptionAdminUpdatedEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `SetRedemptionAdmin`.",
        "- `new_redemption_admin`: Public key of the new redemption admin."
      ],
      "discriminator": [
        122,
        95,
        205,
        126,
        218,
        93,
        18,
        173
      ],
      "accounts": [
        {
          "name": "state",
          "docs": [
            "Program state account containing the redemption admin configuration",
            "",
            "Must be mutable to allow redemption admin updates and have the boss account",
            "as the authorized signer for redemption admin configuration management."
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
            "The boss account authorized to configure the redemption admin"
          ],
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "newRedemptionAdmin",
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
          "name": "offer",
          "docs": [
            "The offer account containing pricing vectors and exchange configuration",
            "",
            "This account is validated as a PDA derived from token mint addresses",
            "and contains the pricing vectors used for dynamic price calculation."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing authorization and kill switch status"
          ],
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
            "The boss account authorized to receive token_in payments",
            "",
            "Must match the boss stored in program state for security validation."
          ],
          "relations": [
            "state"
          ]
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "Program-derived authority that controls vault token operations",
            "",
            "This PDA manages token transfers and burning operations for the",
            "burn/mint architecture when program has mint authority."
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
            "Vault account for temporary token_in storage during burn operations",
            "",
            "Used for burning input tokens when the program has mint authority",
            "for efficient burn/mint token exchange architecture."
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
            "Vault account for token_out distribution when using transfer mechanism",
            "",
            "Source of output tokens when the program lacks mint authority",
            "and must transfer from pre-funded vault instead of minting."
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
            "Input token mint account for the exchange",
            "",
            "Must be mutable to allow burning operations when program has mint authority.",
            "Validated against the offer's expected token_in_mint."
          ],
          "writable": true
        },
        {
          "name": "tokenInProgram",
          "docs": [
            "Token program interface for input token operations"
          ]
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "Output token mint account for the exchange",
            "",
            "Must be mutable to allow minting operations when program has mint authority.",
            "Validated against the offer's expected token_out_mint."
          ],
          "writable": true
        },
        {
          "name": "tokenOutProgram",
          "docs": [
            "Token program interface for output token operations"
          ]
        },
        {
          "name": "userTokenInAccount",
          "docs": [
            "User's input token account for payment",
            "",
            "Source account from which the user pays token_in for the exchange.",
            "Must have sufficient balance for the requested token_in_amount."
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
            "User's output token account for receiving exchanged tokens",
            "",
            "Destination account where the user receives token_out from the exchange.",
            "Created automatically if it doesn't exist using init_if_needed."
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
            "Boss's input token account for receiving payments",
            "",
            "Destination account where the boss receives token_in payments",
            "from users taking offers."
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
          "name": "mintAuthority",
          "docs": [
            "Program-derived mint authority for direct token minting",
            "",
            "Used when the program has mint authority and can mint token_out",
            "directly to users instead of transferring from vault."
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
          "name": "instructionsSysvar",
          "docs": [
            "Instructions sysvar for approval signature verification",
            "",
            "Required for cryptographic verification of approval messages",
            "when offers require boss approval for execution."
          ],
          "address": "Sysvar1nstructions1111111111111111111111111"
        },
        {
          "name": "user",
          "docs": [
            "The user executing the offer and paying for account creation"
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
          "name": "tokenInAmount",
          "type": "u64"
        },
        {
          "name": "approvalMessage",
          "type": {
            "option": {
              "defined": {
                "name": "approvalMessage"
              }
            }
          }
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
          "name": "offer",
          "docs": [
            "The offer account containing pricing vectors and configuration",
            "",
            "Must have allow_permissionless enabled for this instruction to succeed.",
            "Contains pricing vectors for dynamic price calculation."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing authorization and kill switch status"
          ],
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
            "The boss account authorized to receive token_in payments",
            "",
            "Must match the boss stored in program state for security validation."
          ]
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "Program-derived authority that controls vault token operations",
            "",
            "This PDA manages token transfers and burning operations for the",
            "burn/mint architecture when program has mint authority."
          ]
        },
        {
          "name": "vaultTokenInAccount",
          "docs": [
            "Vault account for temporary token_in storage during burn operations",
            "",
            "Used for burning input tokens when the program has mint authority",
            "for efficient burn/mint token exchange architecture."
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
            "Vault account for token_out distribution when using transfer mechanism",
            "",
            "Source of output tokens when the program lacks mint authority",
            "and must transfer from pre-funded vault instead of minting."
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
            "Program-derived authority that controls intermediary token routing accounts",
            "",
            "This PDA manages the intermediary accounts used for permissionless token",
            "routing, enabling secure transfers without direct user-boss relationships."
          ]
        },
        {
          "name": "permissionlessTokenInAccount",
          "docs": [
            "Intermediary account for routing token_in payments",
            "",
            "Temporary holding account that receives user payments before forwarding",
            "to boss, enabling permissionless operations without direct relationships."
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
            "Intermediary account for routing token_out distributions",
            "",
            "Temporary holding account that receives output tokens before forwarding",
            "to user, completing the permissionless routing mechanism."
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
            "Input token mint account for the exchange",
            "",
            "Must be mutable to allow burning operations when program has mint authority.",
            "Validated against the offer's expected token_in_mint."
          ],
          "writable": true
        },
        {
          "name": "tokenInProgram",
          "docs": [
            "Token program interface for input token operations"
          ]
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "Output token mint account for the exchange",
            "",
            "Must be mutable to allow minting operations when program has mint authority.",
            "Validated against the offer's expected token_out_mint."
          ],
          "writable": true
        },
        {
          "name": "tokenOutProgram",
          "docs": [
            "Token program interface for output token operations"
          ]
        },
        {
          "name": "userTokenInAccount",
          "docs": [
            "User's input token account for payment",
            "",
            "Source account from which the user pays token_in for the exchange.",
            "Must have sufficient balance for the requested token_in_amount."
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
            "User's output token account for receiving exchanged tokens",
            "",
            "Destination account where the user receives token_out from the exchange.",
            "Created automatically if it doesn't exist using init_if_needed."
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
            "Boss's input token account for receiving payments",
            "",
            "Final destination account where the boss receives token_in payments",
            "from users taking offers via intermediary routing."
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
          "name": "mintAuthority",
          "docs": [
            "Program-derived mint authority for direct token minting",
            "",
            "Used when the program has mint authority and can mint token_out",
            "directly instead of transferring from vault."
          ]
        },
        {
          "name": "instructionsSysvar",
          "docs": [
            "Instructions sysvar for approval signature verification",
            "",
            "Required for cryptographic verification of approval messages",
            "when offers require boss approval for execution."
          ],
          "address": "Sysvar1nstructions1111111111111111111111111"
        },
        {
          "name": "user",
          "docs": [
            "The user executing the offer and paying for account creation"
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
          "name": "tokenInAmount",
          "type": "u64"
        },
        {
          "name": "approvalMessage",
          "type": {
            "option": {
              "defined": {
                "name": "approvalMessage"
              }
            }
          }
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
            "The boss account authorized to recover mint authority",
            "",
            "Must be the current boss stored in program state and sign the transaction",
            "to authorize the mint authority transfer."
          ],
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing boss validation"
          ],
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
          "name": "mint",
          "docs": [
            "The token mint whose authority will be transferred to the boss",
            "",
            "Must currently have the program PDA as its mint authority. The mint",
            "will be updated to have the boss as the new mint authority."
          ],
          "writable": true
        },
        {
          "name": "mintAuthority",
          "docs": [
            "Program-derived account that currently holds mint authority",
            "",
            "This PDA must be the current mint authority for the token. The program",
            "uses this PDA's signature to authorize transferring authority to the boss."
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
            "The boss account authorized to transfer mint authority",
            "",
            "Must be the current boss stored in program state and currently hold",
            "mint authority for the specified token."
          ],
          "signer": true,
          "relations": [
            "state"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing boss validation"
          ],
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
          "name": "mint",
          "docs": [
            "The token mint whose authority will be transferred to the program",
            "",
            "Must currently have the boss as its mint authority. After the transfer,",
            "the program PDA will be able to mint tokens programmatically."
          ],
          "writable": true
        },
        {
          "name": "mintAuthority",
          "docs": [
            "Program-derived account that will become the new mint authority",
            "",
            "This PDA will receive mint authority and enable the program to mint",
            "tokens directly for controlled supply management operations."
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
          "name": "offer",
          "docs": [
            "The offer account whose fee will be updated",
            "",
            "This account is validated as a PDA derived from token mint addresses",
            "and contains the fee configuration to be modified."
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
                  114
                ]
              },
              {
                "kind": "account",
                "path": "tokenInMint"
              },
              {
                "kind": "account",
                "path": "tokenOutMint"
              }
            ]
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The input token mint account for offer validation"
          ]
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "The output token mint account for offer validation"
          ]
        },
        {
          "name": "state",
          "docs": [
            "Program state account containing boss authorization"
          ],
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
            "The boss account authorized to update offer fees"
          ],
          "signer": true,
          "relations": [
            "state"
          ]
        }
      ],
      "args": [
        {
          "name": "newFeeBasisPoints",
          "type": "u16"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "offer",
      "discriminator": [
        215,
        88,
        60,
        71,
        170,
        162,
        73,
        229
      ]
    },
    {
      "name": "permissionlessAuthority",
      "discriminator": [
        241,
        34,
        5,
        97,
        43,
        102,
        149,
        52
      ]
    },
    {
      "name": "redemptionOffer",
      "discriminator": [
        170,
        229,
        178,
        15,
        184,
        107,
        140,
        41
      ]
    },
    {
      "name": "redemptionRequest",
      "discriminator": [
        117,
        157,
        214,
        214,
        64,
        160,
        31,
        58
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
    },
    {
      "name": "userNonceAccount",
      "discriminator": [
        199,
        159,
        144,
        48,
        120,
        187,
        62,
        2
      ]
    }
  ],
  "events": [
    {
      "name": "adminAddedEvent",
      "discriminator": [
        68,
        183,
        187,
        200,
        190,
        214,
        20,
        77
      ]
    },
    {
      "name": "adminRemovedEvent",
      "discriminator": [
        226,
        5,
        12,
        53,
        69,
        56,
        172,
        132
      ]
    },
    {
      "name": "adminsClearedEvent",
      "discriminator": [
        202,
        81,
        156,
        32,
        66,
        208,
        159,
        153
      ]
    },
    {
      "name": "approverAddedEvent",
      "discriminator": [
        130,
        197,
        173,
        181,
        53,
        38,
        162,
        134
      ]
    },
    {
      "name": "approverRemovedEvent",
      "discriminator": [
        234,
        1,
        25,
        206,
        97,
        119,
        7,
        23
      ]
    },
    {
      "name": "bossAcceptedEvent",
      "discriminator": [
        11,
        133,
        76,
        152,
        219,
        5,
        220,
        103
      ]
    },
    {
      "name": "bossProposedEvent",
      "discriminator": [
        22,
        117,
        195,
        6,
        169,
        57,
        141,
        17
      ]
    },
    {
      "name": "getApyEvent",
      "discriminator": [
        235,
        74,
        195,
        163,
        16,
        198,
        159,
        61
      ]
    },
    {
      "name": "getCirculatingSupplyEvent",
      "discriminator": [
        2,
        255,
        109,
        150,
        90,
        242,
        104,
        206
      ]
    },
    {
      "name": "getNavEvent",
      "discriminator": [
        112,
        70,
        141,
        221,
        181,
        134,
        99,
        92
      ]
    },
    {
      "name": "getNavAdjustmentEvent",
      "discriminator": [
        22,
        137,
        159,
        134,
        238,
        37,
        111,
        158
      ]
    },
    {
      "name": "getTvlEvent",
      "discriminator": [
        12,
        82,
        39,
        27,
        40,
        162,
        216,
        88
      ]
    },
    {
      "name": "killSwitchToggledEvent",
      "discriminator": [
        104,
        2,
        90,
        20,
        64,
        132,
        228,
        122
      ]
    },
    {
      "name": "maxSupplyConfiguredEvent",
      "discriminator": [
        180,
        54,
        16,
        115,
        92,
        70,
        168,
        123
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
      "name": "oNycMintUpdatedEvent",
      "discriminator": [
        221,
        248,
        176,
        184,
        134,
        249,
        29,
        1
      ]
    },
    {
      "name": "offerClosedEvent",
      "discriminator": [
        52,
        107,
        250,
        206,
        40,
        209,
        249,
        150
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
      "name": "offerTakenEvent",
      "discriminator": [
        64,
        121,
        49,
        21,
        184,
        132,
        139,
        54
      ]
    },
    {
      "name": "offerTakenPermissionlessEvent",
      "discriminator": [
        201,
        45,
        242,
        200,
        95,
        48,
        126,
        143
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
      "name": "offerVectorEvictedEvent",
      "discriminator": [
        52,
        231,
        183,
        68,
        181,
        24,
        100,
        243
      ]
    },
    {
      "name": "onycTokensMintedEvent",
      "discriminator": [
        241,
        171,
        63,
        134,
        122,
        8,
        178,
        120
      ]
    },
    {
      "name": "redemptionAdminUpdatedEvent",
      "discriminator": [
        110,
        117,
        47,
        219,
        133,
        230,
        199,
        178
      ]
    },
    {
      "name": "redemptionOfferCreatedEvent",
      "discriminator": [
        171,
        25,
        200,
        106,
        108,
        123,
        70,
        65
      ]
    },
    {
      "name": "redemptionRequestCancelledEvent",
      "discriminator": [
        51,
        146,
        195,
        92,
        134,
        230,
        73,
        134
      ]
    },
    {
      "name": "redemptionRequestCreatedEvent",
      "discriminator": [
        30,
        61,
        76,
        2,
        36,
        82,
        84,
        201
      ]
    },
    {
      "name": "redemptionVaultDepositEvent",
      "discriminator": [
        229,
        187,
        130,
        152,
        236,
        139,
        239,
        108
      ]
    },
    {
      "name": "redemptionVaultWithdrawEvent",
      "discriminator": [
        255,
        92,
        199,
        233,
        33,
        6,
        21,
        78
      ]
    },
    {
      "name": "stateClosedEvent",
      "discriminator": [
        205,
        52,
        85,
        250,
        177,
        119,
        155,
        198
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "expired",
      "msg": "The approval message has expired."
    },
    {
      "code": 6001,
      "name": "wrongProgram",
      "msg": "The approval message is for the wrong program."
    },
    {
      "code": 6002,
      "name": "wrongUser",
      "msg": "The approval message is for the wrong user."
    },
    {
      "code": 6003,
      "name": "missingEd25519Ix",
      "msg": "Missing Ed25519 instruction."
    },
    {
      "code": 6004,
      "name": "wrongIxProgram",
      "msg": "The instruction is for the wrong program."
    },
    {
      "code": 6005,
      "name": "malformedEd25519Ix",
      "msg": "Malformed Ed25519 instruction."
    },
    {
      "code": 6006,
      "name": "multipleSigs",
      "msg": "Multiple signatures found in Ed25519 instruction."
    },
    {
      "code": 6007,
      "name": "wrongAuthority",
      "msg": "The authority public key does not match."
    },
    {
      "code": 6008,
      "name": "msgMismatch",
      "msg": "The message in the Ed25519 instruction does not match the approval message."
    },
    {
      "code": 6009,
      "name": "msgDeserialize",
      "msg": "Failed to deserialize the approval message."
    }
  ],
  "types": [
    {
      "name": "adminAddedEvent",
      "docs": [
        "Event emitted when a new admin is successfully added",
        "",
        "Provides transparency for tracking admin privilege changes."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "docs": [
              "The public key of the newly added admin"
            ],
            "type": "pubkey"
          },
          {
            "name": "boss",
            "docs": [
              "The boss who added the admin"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "adminRemovedEvent",
      "docs": [
        "Event emitted when an admin is successfully removed",
        "",
        "Provides transparency for tracking admin privilege changes."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "docs": [
              "The public key of the removed admin"
            ],
            "type": "pubkey"
          },
          {
            "name": "boss",
            "docs": [
              "The boss who removed the admin"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "adminsClearedEvent",
      "docs": [
        "Event emitted when all admins are successfully cleared",
        "",
        "Provides transparency for tracking admin privilege changes."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "boss",
            "docs": [
              "The boss who cleared all admins"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "approvalMessage",
      "docs": [
        "Message structure for approval verification",
        "",
        "This structure contains the data required to verify that a user has received",
        "approval from a trusted authority to perform a specific action within the program.",
        "The message is signed by the trusted authority using Ed25519 signature.",
        "",
        "# Fields",
        "- `program_id`: The ID of the program for which this approval is valid",
        "- `user_pubkey`: The public key of the user who is approved to perform the action",
        "- `expiry_unix`: Unix timestamp when this approval expires"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "programId",
            "docs": [
              "The program ID this approval is valid for"
            ],
            "type": "pubkey"
          },
          {
            "name": "userPubkey",
            "docs": [
              "The user public key that is approved"
            ],
            "type": "pubkey"
          },
          {
            "name": "expiryUnix",
            "docs": [
              "Unix timestamp when this approval expires"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "approverAddedEvent",
      "docs": [
        "Event emitted when an approver is successfully added",
        "",
        "Provides transparency for tracking approver changes."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "approver",
            "docs": [
              "The public key of the newly added approver"
            ],
            "type": "pubkey"
          },
          {
            "name": "boss",
            "docs": [
              "The boss who added the approver"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "approverRemovedEvent",
      "docs": [
        "Event emitted when an approver is successfully removed",
        "",
        "Provides transparency for tracking approver changes."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "approver",
            "docs": [
              "The public key of the removed approver"
            ],
            "type": "pubkey"
          },
          {
            "name": "boss",
            "docs": [
              "The boss who removed the approver"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "bossAcceptedEvent",
      "docs": [
        "Event emitted when the boss authority is successfully transferred",
        "",
        "Provides transparency for tracking ownership transfers and authority changes."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "oldBoss",
            "docs": [
              "The previous boss's public key before the update"
            ],
            "type": "pubkey"
          },
          {
            "name": "newBoss",
            "docs": [
              "The new boss's public key after the update"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "bossProposedEvent",
      "docs": [
        "Event emitted when a new boss is proposed",
        "",
        "Provides transparency for tracking ownership transfer proposals."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "currentBoss",
            "docs": [
              "The current boss's public key"
            ],
            "type": "pubkey"
          },
          {
            "name": "proposedBoss",
            "docs": [
              "The proposed new boss's public key"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "getApyEvent",
      "docs": [
        "Event emitted when APY calculation is successfully completed",
        "",
        "This event provides transparency for off-chain applications to track",
        "APY queries and monitor yield calculation results for specific offers."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerPda",
            "docs": [
              "The PDA address of the offer for which APY was calculated"
            ],
            "type": "pubkey"
          },
          {
            "name": "apy",
            "docs": [
              "Calculated Annual Percentage Yield with scale=6 (1_000_000 = 100%)"
            ],
            "type": "u64"
          },
          {
            "name": "apr",
            "docs": [
              "Source Annual Percentage Rate with scale=6 used for calculation"
            ],
            "type": "u64"
          },
          {
            "name": "timestamp",
            "docs": [
              "Unix timestamp when the APY calculation was performed"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "getCirculatingSupplyEvent",
      "docs": [
        "Event emitted when circulating supply calculation is completed",
        "",
        "Provides transparency for tracking token supply distribution and vault holdings."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "circulatingSupply",
            "docs": [
              "Calculated circulating supply (total_supply - vault_amount) in base units"
            ],
            "type": "u64"
          },
          {
            "name": "totalSupply",
            "docs": [
              "Total token supply from the mint account in base units"
            ],
            "type": "u64"
          },
          {
            "name": "vaultAmount",
            "docs": [
              "Vault token amount excluded from circulation in base units"
            ],
            "type": "u64"
          },
          {
            "name": "timestamp",
            "docs": [
              "Unix timestamp when the calculation was performed"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "getNavEvent",
      "docs": [
        "Event emitted when NAV (Net Asset Value) calculation is completed",
        "",
        "Provides transparency for tracking current pricing information for offers."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerPda",
            "docs": [
              "The PDA address of the offer for which NAV was calculated"
            ],
            "type": "pubkey"
          },
          {
            "name": "currentPrice",
            "docs": [
              "Current price with 9 decimal precision (scale=9)"
            ],
            "type": "u64"
          },
          {
            "name": "timestamp",
            "docs": [
              "Unix timestamp when the price calculation was performed"
            ],
            "type": "u64"
          },
          {
            "name": "nextPriceChangeTimestamp",
            "docs": [
              "Unix timestamp when the next price change will occur"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "getNavAdjustmentEvent",
      "docs": [
        "Event emitted when NAV adjustment calculation is completed",
        "",
        "Provides transparency for tracking price changes between pricing vectors."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerPda",
            "docs": [
              "The PDA address of the offer for which adjustment was calculated"
            ],
            "type": "pubkey"
          },
          {
            "name": "currentPrice",
            "docs": [
              "Current price from the active vector with scale=9"
            ],
            "type": "u64"
          },
          {
            "name": "previousPrice",
            "docs": [
              "Previous price from the previous vector with scale=9 (None if first vector)"
            ],
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "adjustment",
            "docs": [
              "Price adjustment (current - previous) as signed value with scale=9"
            ],
            "type": "i64"
          },
          {
            "name": "timestamp",
            "docs": [
              "Unix timestamp when the adjustment calculation was performed"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "getTvlEvent",
      "docs": [
        "Event emitted when TVL (Total Value Locked) calculation is completed",
        "",
        "Provides transparency for tracking total value metrics for offers."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerPda",
            "docs": [
              "The PDA address of the offer for which TVL was calculated"
            ],
            "type": "pubkey"
          },
          {
            "name": "tvl",
            "docs": [
              "Calculated TVL in base units (circulating_supply * current_price / 10^9)"
            ],
            "type": "u64"
          },
          {
            "name": "currentPrice",
            "docs": [
              "Current price with scale=9 used for TVL calculation"
            ],
            "type": "u64"
          },
          {
            "name": "tokenSupply",
            "docs": [
              "Circulating token supply (total_supply - vault_amount) in base units"
            ],
            "type": "u64"
          },
          {
            "name": "timestamp",
            "docs": [
              "Unix timestamp when the TVL calculation was performed"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "killSwitchToggledEvent",
      "docs": [
        "Event emitted when the kill switch state is changed",
        "",
        "Provides transparency for tracking emergency control changes."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "enabled",
            "docs": [
              "Whether the kill switch was enabled (true) or disabled (false)"
            ],
            "type": "bool"
          },
          {
            "name": "signer",
            "docs": [
              "The account that toggled the kill switch"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "maxSupplyConfiguredEvent",
      "docs": [
        "Event emitted when the ONyc maximum supply is successfully configured",
        "",
        "Provides transparency for tracking max supply configuration changes."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "oldMaxSupply",
            "docs": [
              "The previous maximum supply cap (0 = no cap)"
            ],
            "type": "u64"
          },
          {
            "name": "newMaxSupply",
            "docs": [
              "The new maximum supply cap (0 = no cap)"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "mintAuthorityTransferredToBossEvent",
      "docs": [
        "Event emitted when mint authority is successfully transferred from program PDA to boss",
        "",
        "Provides transparency for tracking mint authority changes and emergency recovery operations."
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
              "The new authority (boss account)"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "mintAuthorityTransferredToProgramEvent",
      "docs": [
        "Event emitted when mint authority is successfully transferred from boss to program PDA",
        "",
        "Provides transparency for tracking mint authority changes and enabling programmatic control."
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
              "The previous authority (boss account)"
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
      "name": "oNycMintUpdatedEvent",
      "docs": [
        "Event emitted when the ONyc token mint is successfully updated",
        "",
        "Provides transparency for tracking ONyc mint configuration changes."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "oldOnycMint",
            "docs": [
              "The previous ONyc mint public key before the update"
            ],
            "type": "pubkey"
          },
          {
            "name": "newOnycMint",
            "docs": [
              "The new ONyc mint public key after the update"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offer",
      "docs": [
        "Token exchange offer with dynamic APR-based pricing",
        "",
        "Stores configuration for token pair exchanges with time-based pricing vectors",
        "that implement compound interest growth using Annual Percentage Rate (APR).",
        "Each offer is unique per token pair and supports up to 10 pricing vectors."
      ],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenInMint",
            "docs": [
              "Input token mint for the exchange"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenOutMint",
            "docs": [
              "Output token mint for the exchange"
            ],
            "type": "pubkey"
          },
          {
            "name": "vectors",
            "docs": [
              "Array of pricing vectors defining price evolution over time"
            ],
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
            "name": "feeBasisPoints",
            "docs": [
              "Fee in basis points (10000 = 100%) charged when taking the offer"
            ],
            "type": "u16"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed for account derivation"
            ],
            "type": "u8"
          },
          {
            "name": "needsApproval",
            "docs": [
              "Whether the offer requires boss approval for taking (0 = false, 1 = true)"
            ],
            "type": "u8"
          },
          {
            "name": "allowPermissionless",
            "docs": [
              "Whether the offer allows permissionless operations (0 = false, 1 = true)"
            ],
            "type": "u8"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved space for future fields"
            ],
            "type": {
              "array": [
                "u8",
                131
              ]
            }
          }
        ]
      }
    },
    {
      "name": "offerClosedEvent",
      "docs": [
        "Event emitted when an offer is successfully closed and account is reclaimed",
        "",
        "Provides transparency for tracking offer lifecycle and account cleanup operations."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerPda",
            "docs": [
              "The PDA address of the offer that was closed"
            ],
            "type": "pubkey"
          },
          {
            "name": "boss",
            "docs": [
              "The boss account that initiated the closure and received the rent"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offerFeeUpdatedEvent",
      "docs": [
        "Event emitted when an offer's fee is successfully updated",
        "",
        "Provides transparency for tracking fee changes and offer configuration modifications."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerPda",
            "docs": [
              "The PDA address of the offer whose fee was updated"
            ],
            "type": "pubkey"
          },
          {
            "name": "oldFeeBasisPoints",
            "docs": [
              "Previous fee in basis points (10000 = 100%)"
            ],
            "type": "u16"
          },
          {
            "name": "newFeeBasisPoints",
            "docs": [
              "New fee in basis points (10000 = 100%)"
            ],
            "type": "u16"
          },
          {
            "name": "boss",
            "docs": [
              "The boss account that authorized the fee update"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offerMadeEvent",
      "docs": [
        "Event emitted when an offer is successfully created",
        "",
        "Provides transparency for tracking offer creation and configuration parameters."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerPda",
            "docs": [
              "The PDA address of the newly created offer"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenInMint",
            "docs": [
              "The input token mint for the offer"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenOutMint",
            "docs": [
              "The output token mint for the offer"
            ],
            "type": "pubkey"
          },
          {
            "name": "feeBasisPoints",
            "docs": [
              "Fee in basis points (10000 = 100%) charged when taking the offer"
            ],
            "type": "u16"
          },
          {
            "name": "boss",
            "docs": [
              "The boss account that created and owns the offer"
            ],
            "type": "pubkey"
          },
          {
            "name": "needsApproval",
            "docs": [
              "Whether the offer requires boss approval for taking"
            ],
            "type": "bool"
          },
          {
            "name": "allowPermissionless",
            "docs": [
              "Whether the offer allows permissionless operations"
            ],
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "offerTakenEvent",
      "docs": [
        "Event emitted when an offer is successfully taken",
        "",
        "Provides transparency for tracking offer execution and token exchange details."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerPda",
            "docs": [
              "The PDA address of the offer that was executed"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenInAmount",
            "docs": [
              "Amount of token_in paid by the user after fee deduction"
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
              "Fee amount deducted from the original token_in payment"
            ],
            "type": "u64"
          },
          {
            "name": "user",
            "docs": [
              "Public key of the user who executed the offer"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offerTakenPermissionlessEvent",
      "docs": [
        "Event emitted when an offer is successfully executed via permissionless flow",
        "",
        "Provides transparency for tracking permissionless offer execution with intermediary routing."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerPda",
            "docs": [
              "The PDA address of the offer that was executed"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenInAmount",
            "docs": [
              "Amount of token_in paid by the user after fee deduction"
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
              "Fee amount deducted from the original token_in payment"
            ],
            "type": "u64"
          },
          {
            "name": "user",
            "docs": [
              "Public key of the user who executed the offer"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offerVaultDepositEvent",
      "docs": [
        "Event emitted when tokens are successfully deposited to the offer vault",
        "",
        "Provides transparency for tracking vault funding and token availability."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "The token mint that was deposited"
            ],
            "type": "pubkey"
          },
          {
            "name": "amount",
            "docs": [
              "Amount of tokens deposited to the vault"
            ],
            "type": "u64"
          },
          {
            "name": "boss",
            "docs": [
              "The boss account that made the deposit"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offerVaultWithdrawEvent",
      "docs": [
        "Event emitted when tokens are successfully withdrawn from the offer vault",
        "",
        "Provides transparency for tracking vault withdrawals and fund management."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "The token mint that was withdrawn"
            ],
            "type": "pubkey"
          },
          {
            "name": "amount",
            "docs": [
              "Amount of tokens withdrawn from the vault"
            ],
            "type": "u64"
          },
          {
            "name": "boss",
            "docs": [
              "The boss account that performed the withdrawal"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "offerVector",
      "docs": [
        "Time-based pricing vector with APR-driven compound growth",
        "",
        "Defines price evolution over time using Annual Percentage Rate (APR) with",
        "discrete pricing steps. Each vector becomes active at start_time and",
        "implements compound interest pricing until the next vector activates."
      ],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "startTime",
            "docs": [
              "Calculated activation time: max(base_time, current_time) when vector was added"
            ],
            "type": "u64"
          },
          {
            "name": "baseTime",
            "docs": [
              "Original requested activation time before current_time adjustment"
            ],
            "type": "u64"
          },
          {
            "name": "basePrice",
            "docs": [
              "Initial price with scale=9 (1_000_000_000 = 1.0) at vector start"
            ],
            "type": "u64"
          },
          {
            "name": "apr",
            "docs": [
              "Annual Percentage Rate scaled by 1_000_000 (1_000_000 = 1% APR)",
              "",
              "Determines compound interest rate for price growth over time.",
              "Scale=6 where 1_000_000 = 1% annual rate."
            ],
            "type": "u64"
          },
          {
            "name": "priceFixDuration",
            "docs": [
              "Duration in seconds for each discrete pricing step"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "offerVectorAddedEvent",
      "docs": [
        "Event emitted when a pricing vector is successfully added to an offer",
        "",
        "Provides transparency for tracking pricing vector additions and configurations."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerPda",
            "docs": [
              "The PDA address of the offer to which the vector was added"
            ],
            "type": "pubkey"
          },
          {
            "name": "startTime",
            "docs": [
              "Calculated start time when the vector becomes active (max(base_time, current_time))"
            ],
            "type": "u64"
          },
          {
            "name": "baseTime",
            "docs": [
              "Original base time specified for the vector"
            ],
            "type": "u64"
          },
          {
            "name": "basePrice",
            "docs": [
              "Base price with 9 decimal precision at the vector start"
            ],
            "type": "u64"
          },
          {
            "name": "apr",
            "docs": [
              "Annual Percentage Rate scaled by 1,000,000 (1_000_000 = 1% APR)"
            ],
            "type": "u64"
          },
          {
            "name": "priceFixDuration",
            "docs": [
              "Duration in seconds for each discrete pricing step"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "offerVectorDeletedEvent",
      "docs": [
        "Event emitted when a pricing vector is successfully deleted from an offer",
        "",
        "Provides transparency for tracking pricing vector removals and offer configuration changes."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerPda",
            "docs": [
              "The PDA address of the offer from which the vector was deleted"
            ],
            "type": "pubkey"
          },
          {
            "name": "vectorStartTime",
            "docs": [
              "Start time of the deleted pricing vector"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "offerVectorEvictedEvent",
      "docs": [
        "Event emitted when old pricing vectors are retired from an offer"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerTokenInMint",
            "docs": [
              "The token in mint of the offer"
            ],
            "type": "pubkey"
          },
          {
            "name": "offerTokenOutMint",
            "docs": [
              "The token out mint of the offer"
            ],
            "type": "pubkey"
          },
          {
            "name": "vectorStartTime",
            "docs": [
              "Start time of the retired pricing vector"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "onycTokensMintedEvent",
      "docs": [
        "Event emitted when ONyc tokens are successfully minted to the boss account",
        "",
        "Provides transparency for tracking token minting operations performed by the boss."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "onycMint",
            "docs": [
              "The ONyc mint from which tokens were minted"
            ],
            "type": "pubkey"
          },
          {
            "name": "boss",
            "docs": [
              "The boss account that received the newly minted tokens"
            ],
            "type": "pubkey"
          },
          {
            "name": "amount",
            "docs": [
              "The amount of tokens minted in base units"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "permissionlessAuthority",
      "docs": [
        "Program-derived authority for permissionless token routing operations",
        "",
        "This PDA manages intermediary accounts used for permissionless offer execution,",
        "enabling secure token routing without direct user-boss relationships."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "name",
            "docs": [
              "Optional name identifier for the authority (max 50 characters)"
            ],
            "type": "string"
          }
        ]
      }
    },
    {
      "name": "redemptionAdminUpdatedEvent",
      "docs": [
        "Event emitted when the redemption admin is successfully updated",
        "",
        "Provides transparency for tracking redemption admin configuration changes."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "oldRedemptionAdmin",
            "docs": [
              "The previous redemption admin public key before the update"
            ],
            "type": "pubkey"
          },
          {
            "name": "newRedemptionAdmin",
            "docs": [
              "The new redemption admin public key after the update"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "redemptionOffer",
      "docs": [
        "Redemption offer for converting ONyc tokens back to stable tokens",
        "",
        "Manages the redemption process where users can exchange ONyc (in-token)",
        "for stable tokens like USDC (out-token) at the current NAV price.",
        "This is the inverse of the standard Offer which exchanges stable tokens for ONyc."
      ],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offer",
            "docs": [
              "Reference to the original Offer PDA that this redemption offer is associated with"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenInMint",
            "docs": [
              "Input token mint for redemptions (must be ONyc mint from State)"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenOutMint",
            "docs": [
              "Output token mint for redemptions (e.g., USDC)"
            ],
            "type": "pubkey"
          },
          {
            "name": "executedRedemptions",
            "docs": [
              "Cumulative total of all executed redemptions over the contract's lifetime",
              "",
              "This tracks the total amount of ONyc that has been redeemed and burned.",
              "Uses u128 because cumulative redemptions can exceed the current total supply."
            ],
            "type": "u128"
          },
          {
            "name": "requestedRedemptions",
            "docs": [
              "Total amount of pending redemption requests",
              "",
              "This tracks ONyc tokens that are locked in pending redemption requests.",
              "Uses u64 because pending redemptions cannot exceed the token's total supply."
            ],
            "type": "u64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed for account derivation"
            ],
            "type": "u8"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved space for future fields"
            ],
            "type": {
              "array": [
                "u8",
                119
              ]
            }
          }
        ]
      }
    },
    {
      "name": "redemptionOfferCreatedEvent",
      "docs": [
        "Event emitted when a redemption offer is successfully created",
        "",
        "Provides transparency for tracking redemption offer creation and configuration."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "redemptionOfferPda",
            "docs": [
              "The PDA address of the newly created redemption offer"
            ],
            "type": "pubkey"
          },
          {
            "name": "offer",
            "docs": [
              "Reference to the original offer"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenInMint",
            "docs": [
              "The input token mint for redemptions (ONyc)"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenOutMint",
            "docs": [
              "The output token mint for redemptions (e.g., USDC)"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "redemptionRequest",
      "serialization": "bytemuck",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offer",
            "docs": [
              "Reference to the RedemptionOffer PDA"
            ],
            "type": "pubkey"
          },
          {
            "name": "redeemer",
            "docs": [
              "User requesting the redemption"
            ],
            "type": "pubkey"
          },
          {
            "name": "amount",
            "docs": [
              "Amount of token_in tokens requested for redemption"
            ],
            "type": "u64"
          },
          {
            "name": "expiresAt",
            "docs": [
              "Unix timestamp when the request expires"
            ],
            "type": "u64"
          },
          {
            "name": "status",
            "docs": [
              "Status of the redemption request",
              "0: Pending, 1: Executed, 2: Cancelled"
            ],
            "type": "u8"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed for account derivation"
            ],
            "type": "u8"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved space for future fields"
            ],
            "type": {
              "array": [
                "u8",
                126
              ]
            }
          }
        ]
      }
    },
    {
      "name": "redemptionRequestCancelledEvent",
      "docs": [
        "Event emitted when a redemption request is successfully cancelled",
        "",
        "Provides transparency for tracking cancelled redemption requests."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "redemptionRequestPda",
            "docs": [
              "The PDA address of the cancelled redemption request"
            ],
            "type": "pubkey"
          },
          {
            "name": "redemptionOffer",
            "docs": [
              "Reference to the redemption offer"
            ],
            "type": "pubkey"
          },
          {
            "name": "redeemer",
            "docs": [
              "User who requested the redemption"
            ],
            "type": "pubkey"
          },
          {
            "name": "amount",
            "docs": [
              "Amount of token_in tokens that was requested for redemption"
            ],
            "type": "u64"
          },
          {
            "name": "cancelledBy",
            "docs": [
              "The signer who cancelled the request"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "redemptionRequestCreatedEvent",
      "docs": [
        "Event emitted when a redemption request is successfully created",
        "",
        "Provides transparency for tracking redemption requests and their configuration."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "redemptionRequestPda",
            "docs": [
              "The PDA address of the newly created redemption request"
            ],
            "type": "pubkey"
          },
          {
            "name": "redemptionOffer",
            "docs": [
              "Reference to the redemption offer"
            ],
            "type": "pubkey"
          },
          {
            "name": "redeemer",
            "docs": [
              "User requesting the redemption"
            ],
            "type": "pubkey"
          },
          {
            "name": "amount",
            "docs": [
              "Amount of token_in tokens requested for redemption"
            ],
            "type": "u64"
          },
          {
            "name": "expiresAt",
            "docs": [
              "Unix timestamp when the request expires"
            ],
            "type": "u64"
          },
          {
            "name": "usedNonce",
            "docs": [
              "Nonce used for this request"
            ],
            "type": "u64"
          },
          {
            "name": "newNonce",
            "docs": [
              "New nonce, which should be used for the next request"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "redemptionVaultDepositEvent",
      "docs": [
        "Event emitted when tokens are successfully deposited to the redemption vault",
        "",
        "Provides transparency for tracking redemption vault funding and token availability."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "The token mint that was deposited"
            ],
            "type": "pubkey"
          },
          {
            "name": "amount",
            "docs": [
              "Amount of tokens deposited to the vault"
            ],
            "type": "u64"
          },
          {
            "name": "boss",
            "docs": [
              "The boss account that made the deposit"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "redemptionVaultWithdrawEvent",
      "docs": [
        "Event emitted when tokens are successfully withdrawn from the redemption vault",
        "",
        "Provides transparency for tracking redemption vault withdrawals and fund management."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "The token mint that was withdrawn"
            ],
            "type": "pubkey"
          },
          {
            "name": "amount",
            "docs": [
              "Amount of tokens withdrawn from the vault"
            ],
            "type": "u64"
          },
          {
            "name": "boss",
            "docs": [
              "The boss account that performed the withdrawal"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "state",
      "docs": [
        "Global program state containing governance and configuration settings",
        "",
        "Stores the core program authority structure, emergency controls, and trusted entities",
        "used for authorization and approval verification across all program operations."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "boss",
            "docs": [
              "Primary program authority with full control over all operations"
            ],
            "type": "pubkey"
          },
          {
            "name": "proposedBoss",
            "docs": [
              "Proposed new boss for two-step ownership transfer"
            ],
            "type": "pubkey"
          },
          {
            "name": "isKilled",
            "docs": [
              "Emergency kill switch to halt critical operations when activated"
            ],
            "type": "bool"
          },
          {
            "name": "onycMint",
            "docs": [
              "ONyc token mint used for market calculations and operations"
            ],
            "type": "pubkey"
          },
          {
            "name": "admins",
            "docs": [
              "Array of admin accounts authorized to enable the kill switch"
            ],
            "type": {
              "array": [
                "pubkey",
                20
              ]
            }
          },
          {
            "name": "approver1",
            "docs": [
              "First trusted authority for cryptographic approval verification"
            ],
            "type": "pubkey"
          },
          {
            "name": "approver2",
            "docs": [
              "Second trusted authority for cryptographic approval verification"
            ],
            "type": "pubkey"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed for account derivation"
            ],
            "type": "u8"
          },
          {
            "name": "maxSupply",
            "docs": [
              "Optional maximum supply cap for ONyc token minting (0 = no cap)"
            ],
            "type": "u64"
          },
          {
            "name": "redemptionAdmin",
            "docs": [
              "Admin account authorized to manage ONr token mints and redemptions"
            ],
            "type": "pubkey"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved space for future program state extensions"
            ],
            "type": {
              "array": [
                "u8",
                96
              ]
            }
          }
        ]
      }
    },
    {
      "name": "stateClosedEvent",
      "docs": [
        "Event emitted when the state account is successfully closed",
        "",
        "Provides transparency for tracking the closure of the program's main state account."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "statePda",
            "docs": [
              "The PDA address of the state account that was closed"
            ],
            "type": "pubkey"
          },
          {
            "name": "boss",
            "docs": [
              "The boss account that initiated the closure and received the rent"
            ],
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "userNonceAccount",
      "docs": [
        "User nonce account for preventing replay attacks.",
        "",
        "Each user has a unique nonce account that is incremented with each successful transaction."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nonce",
            "type": "u64"
          }
        ]
      }
    }
  ]
};
