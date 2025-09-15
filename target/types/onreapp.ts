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
      "name": "addBuyOfferVector",
      "docs": [
        "Adds a time vector to an existing buy offer.",
        "",
        "Delegates to `buy_offer::add_buy_offer_time_vector`.",
        "Creates a new time vector with auto-generated vector_id for the specified buy offer.",
        "Emits a `BuyOfferVectorAdded` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `AddBuyOfferVector`.",
        "- `offer_id`: ID of the buy offer to add the vector to.",
        "- `base_time`: Unix timestamp when the vector becomes active.",
        "- `base_price`: Price at the beginning of the vector.",
        "- `apr`: Annual Percentage Rate (APR) (see BuyOfferVector::apr for details).",
        "- `price_fix_duration`: Duration in seconds for each price interval."
      ],
      "discriminator": [
        246,
        20,
        14,
        161,
        143,
        108,
        10,
        4
      ],
      "accounts": [
        {
          "name": "buyOfferAccount",
          "docs": [
            "The buy offer account containing all buy offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  95,
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
      "name": "buyOfferVaultDeposit",
      "docs": [
        "Deposits tokens into the buy offer vault.",
        "",
        "Delegates to `vault_operations::buy_offer_vault_deposit`.",
        "Transfers tokens from boss's account to buy offer vault's token account for the specified mint.",
        "Creates vault token account if it doesn't exist using init_if_needed.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `BuyOfferVaultDeposit`.",
        "- `amount`: Amount of tokens to deposit."
      ],
      "discriminator": [
        10,
        253,
        50,
        184,
        56,
        17,
        2,
        50
      ],
      "accounts": [
        {
          "name": "vaultAuthority",
          "docs": [
            "The buy offer vault authority account that controls the vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
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
            "SPL Token program."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
      "name": "buyOfferVaultWithdraw",
      "docs": [
        "Withdraws tokens from the buy offer vault.",
        "",
        "Delegates to `vault_operations::buy_offer_vault_withdraw`.",
        "Transfers tokens from buy offer vault's token account to boss's account for the specified mint.",
        "Creates boss token account if it doesn't exist using init_if_needed.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `BuyOfferVaultWithdraw`.",
        "- `amount`: Amount of tokens to withdraw."
      ],
      "discriminator": [
        32,
        40,
        220,
        58,
        225,
        251,
        124,
        211
      ],
      "accounts": [
        {
          "name": "vaultAuthority",
          "docs": [
            "The buy offer vault authority account that controls the vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
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
            "SPL Token program."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
      "name": "closeBuyOffer",
      "docs": [
        "Closes a buy offer.",
        "",
        "Delegates to `buy_offer::close_buy_offer`.",
        "Removes the offer from the buy offers account and clears its data.",
        "Emits a `CloseBuyOfferEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `CloseBuyOffer`.",
        "- `offer_id`: ID of the offer to close."
      ],
      "discriminator": [
        219,
        41,
        39,
        0,
        56,
        98,
        212,
        133
      ],
      "accounts": [
        {
          "name": "buyOfferAccount",
          "docs": [
            "The buy offer account within the BuyOfferAccount, rent paid by `boss`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  95,
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
      "name": "closeDualRedemptionOffer",
      "docs": [
        "Closes a dual redemption offer.",
        "",
        "Delegates to `redemption_offer::close_dual_redemption_offer`.",
        "Removes the offer from the dual redemption offers account and clears its data.",
        "Only the boss can close dual redemption offers.",
        "Emits a `CloseDualRedemptionOfferEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `CloseDualRedemptionOffer`.",
        "- `offer_id`: ID of the offer to close."
      ],
      "discriminator": [
        119,
        28,
        211,
        241,
        93,
        126,
        181,
        254
      ],
      "accounts": [
        {
          "name": "dualRedemptionOfferAccount",
          "docs": [
            "The dual redemption offer account containing all offers."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  117,
                  97,
                  108,
                  95,
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
      "name": "closeSingleRedemptionOffer",
      "docs": [
        "Closes a single redemption offer.",
        "",
        "Delegates to `redemption_offer::close_single_redemption_offer`.",
        "Removes the offer from the single redemption offers account and clears its data.",
        "Emits a `CloseSingleRedemptionOfferEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `CloseSingleRedemptionOffer`.",
        "- `offer_id`: ID of the offer to close."
      ],
      "discriminator": [
        170,
        162,
        152,
        127,
        143,
        20,
        36,
        234
      ],
      "accounts": [
        {
          "name": "singleRedemptionOfferAccount",
          "docs": [
            "The single redemption offer account within the SingleRedemptionOfferAccount, rent paid by `boss`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  105,
                  110,
                  103,
                  108,
                  101,
                  95,
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
      "name": "deleteBuyOfferVector",
      "docs": [
        "Deletes a time vector from a buy offer.",
        "",
        "Delegates to `buy_offer::delete_buy_offer_vector`.",
        "Removes the specified time vector from the buy offer by setting it to default values.",
        "Only the boss can delete time vectors from offers.",
        "Emits a `BuyOfferVectorDeleted` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `DeleteBuyOfferVector`.",
        "- `offer_id`: ID of the buy offer containing the vector to delete.",
        "- `vector_id`: ID of the vector to delete."
      ],
      "discriminator": [
        37,
        168,
        3,
        196,
        118,
        93,
        9,
        204
      ],
      "accounts": [
        {
          "name": "buyOfferAccount",
          "docs": [
            "The buy offer account containing all buy offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  95,
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
      "name": "dualRedemptionVaultDeposit",
      "docs": [
        "Deposits tokens into the dual redemption vault.",
        "",
        "Delegates to `vault_operations::dual_redemption_vault_deposit`.",
        "Transfers tokens from boss's account to dual redemption vault's token account for the specified mint.",
        "Creates vault token account if it doesn't exist using init_if_needed.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `DualRedemptionVaultDeposit`.",
        "- `amount`: Amount of tokens to deposit."
      ],
      "discriminator": [
        40,
        38,
        237,
        177,
        95,
        79,
        162,
        234
      ],
      "accounts": [
        {
          "name": "vaultAuthority",
          "docs": [
            "The dual redemption vault authority account that controls the vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  117,
                  97,
                  108,
                  95,
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
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
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
            "SPL Token program."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
      "name": "dualRedemptionVaultWithdraw",
      "docs": [
        "Withdraws tokens from the dual redemption vault.",
        "",
        "Delegates to `vault_operations::dual_redemption_vault_withdraw`.",
        "Transfers tokens from dual redemption vault's token account to boss's account for the specified mint.",
        "Creates boss token account if it doesn't exist using init_if_needed.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `DualRedemptionVaultWithdraw`.",
        "- `amount`: Amount of tokens to withdraw."
      ],
      "discriminator": [
        54,
        69,
        13,
        189,
        172,
        165,
        182,
        145
      ],
      "accounts": [
        {
          "name": "vaultAuthority",
          "docs": [
            "The dual redemption vault authority account that controls the vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  117,
                  97,
                  108,
                  95,
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
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
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
            "SPL Token program."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
      "name": "initializeOffers",
      "docs": [
        "Initializes the buy offers account.",
        "",
        "Delegates to `buy_offer::initialize_offers`.",
        "Only the boss can call this instruction to create the buy offers account.",
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
          "name": "buyOfferAccount",
          "docs": [
            "The buy offer account to initialize, rent paid by `boss`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  95,
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
          "name": "singleRedemptionOfferAccount",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  105,
                  110,
                  103,
                  108,
                  101,
                  95,
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
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "dualRedemptionOfferAccount",
          "docs": [
            "The dual redemption offer account to initialize, rent paid by `boss`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  117,
                  97,
                  108,
                  95,
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
                  98,
                  117,
                  121,
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
          "name": "singleRedemptionVaultAuthority",
          "docs": [
            "The single redemption vault authority account to initialize, rent paid by `boss`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  105,
                  110,
                  103,
                  108,
                  101,
                  95,
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
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
                ]
              }
            ]
          }
        },
        {
          "name": "dualRedemptionVaultAuthority",
          "docs": [
            "The dual redemption vault authority account to initialize, rent paid by `boss`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  117,
                  97,
                  108,
                  95,
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
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
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
      "name": "makeBuyOffer",
      "docs": [
        "Creates a buy offer.",
        "",
        "Delegates to `buy_offer::make_buy_offer`.",
        "The price of the token_out changes over time based on `base_price`,",
        "`end_price`, and `price_fix_duration` within the offer's active time window.",
        "Emits a `BuyOfferMade` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `MakeBuyOffer`.",
        "- `fee_basis_points`: Fee in basis points (e.g., 500 = 5%) charged when taking the offer."
      ],
      "discriminator": [
        252,
        213,
        23,
        83,
        192,
        243,
        197,
        78
      ],
      "accounts": [
        {
          "name": "buyOfferAccount",
          "docs": [
            "The buy offer account within the BuyOfferAccount, rent paid by `boss`. Already initialized in initialize."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  95,
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
            "The buy offer vault authority PDA that controls vault token accounts"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
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
            "Mint of the token_in for the offer."
          ]
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
          "name": "tokenProgram",
          "docs": [
            "SPL Token program for token transfers"
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
      "name": "makeDualRedemptionOffer",
      "docs": [
        "Creates a dual redemption offer.",
        "",
        "Delegates to `redemption_offer::make_dual_redemption_offer`.",
        "Creates an offer where users can exchange token_in for two different token_out at fixed prices.",
        "The ratio_basis_points determines the split between the two output tokens.",
        "Emits a `DualRedemptionOfferMadeEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `MakeDualRedemptionOffer`.",
        "- `start_time`: Unix timestamp for when the offer becomes active.",
        "- `end_time`: Unix timestamp for when the offer expires.",
        "- `price_1`: Fixed price for token_out_1 with 9 decimal precision.",
        "- `price_2`: Fixed price for token_out_2 with 9 decimal precision.",
        "- `ratio_basis_points`: Ratio in basis points for token_out_1 (e.g., 8000 = 80% for token_out_1, 20% for token_out_2).",
        "- `fee_basis_points`: Fee in basis points (e.g., 500 = 5%) charged when taking the offer."
      ],
      "discriminator": [
        118,
        199,
        200,
        41,
        17,
        39,
        110,
        2
      ],
      "accounts": [
        {
          "name": "dualRedemptionOfferAccount",
          "docs": [
            "The dual redemption offer account within the DualRedemptionOfferAccount, rent paid by `boss`. Already initialized in initialize."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  117,
                  97,
                  108,
                  95,
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
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "The single redemption vault authority that controls vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  117,
                  97,
                  108,
                  95,
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
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
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
          "name": "tokenOutMint1",
          "docs": [
            "Mint of the first token_out for the offer."
          ]
        },
        {
          "name": "tokenOutMint2",
          "docs": [
            "Mint of the second token_out for the offer."
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
          "name": "tokenProgram",
          "docs": [
            "SPL Token program for token transfers"
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
          "name": "startTime",
          "type": "u64"
        },
        {
          "name": "endTime",
          "type": "u64"
        },
        {
          "name": "price1",
          "type": "u64"
        },
        {
          "name": "price2",
          "type": "u64"
        },
        {
          "name": "ratioBasisPoints",
          "type": "u64"
        },
        {
          "name": "feeBasisPoints",
          "type": "u64"
        }
      ]
    },
    {
      "name": "makeSingleRedemptionOffer",
      "docs": [
        "Creates a single redemption offer.",
        "",
        "Delegates to `redemption_offer::make_single_redemption_offer`.",
        "Creates an offer where users can exchange token_in for token_out at a fixed price.",
        "Emits a `SingleRedemptionOfferMadeEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `MakeSingleRedemptionOffer`.",
        "- `start_time`: Unix timestamp for when the offer becomes active.",
        "- `end_time`: Unix timestamp for when the offer expires.",
        "- `price`: How much token_in needed for 1 token_out, with 9 decimal precision.",
        "- `fee_basis_points`: Fee in basis points (e.g., 500 = 5%) charged when taking the offer."
      ],
      "discriminator": [
        166,
        32,
        26,
        139,
        15,
        237,
        70,
        155
      ],
      "accounts": [
        {
          "name": "singleRedemptionOfferAccount",
          "docs": [
            "The single redemption offer account within the SingleRedemptionOfferAccount, rent paid by `boss`. Already initialized in initialize."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  105,
                  110,
                  103,
                  108,
                  101,
                  95,
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
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "The single redemption vault authority that controls vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  105,
                  110,
                  103,
                  108,
                  101,
                  95,
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
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
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
          "name": "tokenProgram",
          "docs": [
            "SPL Token program for token transfers"
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
          "name": "startTime",
          "type": "u64"
        },
        {
          "name": "endTime",
          "type": "u64"
        },
        {
          "name": "price",
          "type": "u64"
        },
        {
          "name": "feeBasisPoints",
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
      "name": "singleRedemptionVaultDeposit",
      "docs": [
        "Deposits tokens into the single redemption vault.",
        "",
        "Delegates to `vault_operations::single_redemption_vault_deposit`.",
        "Transfers tokens from boss's account to single redemption vault's token account for the specified mint.",
        "Creates vault token account if it doesn't exist using init_if_needed.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `SingleRedemptionVaultDeposit`.",
        "- `amount`: Amount of tokens to deposit."
      ],
      "discriminator": [
        114,
        140,
        19,
        83,
        244,
        253,
        166,
        169
      ],
      "accounts": [
        {
          "name": "vaultAuthority",
          "docs": [
            "The single redemption vault authority account that controls the vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  105,
                  110,
                  103,
                  108,
                  101,
                  95,
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
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
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
            "SPL Token program."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
      "name": "singleRedemptionVaultWithdraw",
      "docs": [
        "Withdraws tokens from the single redemption vault.",
        "",
        "Delegates to `vault_operations::single_redemption_vault_withdraw`.",
        "Transfers tokens from single redemption vault's token account to boss's account for the specified mint.",
        "Creates boss token account if it doesn't exist using init_if_needed.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `SingleRedemptionVaultWithdraw`.",
        "- `amount`: Amount of tokens to withdraw."
      ],
      "discriminator": [
        195,
        57,
        229,
        65,
        216,
        121,
        237,
        77
      ],
      "accounts": [
        {
          "name": "vaultAuthority",
          "docs": [
            "The single redemption vault authority account that controls the vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  105,
                  110,
                  103,
                  108,
                  101,
                  95,
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
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
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
            "SPL Token program."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
      "name": "takeBuyOffer",
      "docs": [
        "Takes a buy offer.",
        "",
        "Delegates to `buy_offer::take_buy_offer`.",
        "Allows a user to exchange token_in for token_out based on the offer's dynamic price.",
        "Emits a `TakeBuyOfferEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `TakeBuyOffer`.",
        "- `offer_id`: ID of the offer to take.",
        "- `token_in_amount`: Amount of token_in to provide."
      ],
      "discriminator": [
        10,
        155,
        119,
        29,
        245,
        1,
        19,
        212
      ],
      "accounts": [
        {
          "name": "buyOfferAccount",
          "docs": [
            "The buy offer account containing all active buy offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  95,
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
            "The buy offer vault authority PDA that controls vault token accounts"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
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
            "The mint account for the input token (what user pays)"
          ]
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
          "name": "user",
          "docs": [
            "The user taking the offer (must sign the transaction)"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "SPL Token program for token operations"
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
      "name": "takeBuyOfferPermissionless",
      "docs": [
        "Takes a buy offer using permissionless flow with intermediary accounts.",
        "",
        "Delegates to `buy_offer::take_buy_offer_permissionless`.",
        "Similar to take_buy_offer but routes token transfers through intermediary accounts",
        "owned by the program instead of direct user-to-boss and vault-to-user transfers.",
        "Emits a `TakeBuyOfferPermissionlessEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `TakeBuyOfferPermissionless`.",
        "- `offer_id`: ID of the offer to take.",
        "- `token_in_amount`: Amount of token_in to provide."
      ],
      "discriminator": [
        209,
        116,
        84,
        124,
        15,
        126,
        86,
        66
      ],
      "accounts": [
        {
          "name": "buyOfferAccount",
          "docs": [
            "The buy offer account containing all active buy offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  95,
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
            "The buy offer vault authority PDA that controls vault token accounts"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
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
            "The mint account for the input token (what user pays)"
          ]
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
          "name": "user",
          "docs": [
            "The user taking the offer (must sign the transaction)"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "SPL Token program for token transfers"
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
      "name": "takeDualRedemptionOffer",
      "docs": [
        "Takes a dual redemption offer.",
        "",
        "Delegates to `redemption_offer::take_dual_redemption_offer`.",
        "Allows a user to exchange token_in for token_out_1 and token_out_2 based on the offer's prices and ratio.",
        "The ratio_basis_points determines how the token_in amount is split between the two output tokens.",
        "Anyone can take the offer as long as it's active and vault has sufficient balances.",
        "Emits a `TakeDualRedemptionOfferEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `TakeDualRedemptionOffer`.",
        "- `offer_id`: ID of the offer to take.",
        "- `token_in_amount`: Amount of token_in to provide."
      ],
      "discriminator": [
        169,
        176,
        120,
        222,
        25,
        181,
        136,
        201
      ],
      "accounts": [
        {
          "name": "dualRedemptionOfferAccount",
          "docs": [
            "The dual redemption offer account containing all offers."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  117,
                  97,
                  108,
                  95,
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
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state to get the boss."
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The boss account that receives token_in payments.",
            "This must match the boss in the state account."
          ]
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "The dual redemption vault authority that controls vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  117,
                  97,
                  108,
                  95,
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
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
                ]
              }
            ]
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The token mint for token_in.",
            "Must be mutable to allow burning when program has mint authority"
          ],
          "writable": true
        },
        {
          "name": "tokenOutMint1",
          "docs": [
            "The token mint for token_out_1."
          ]
        },
        {
          "name": "tokenOutMint2",
          "docs": [
            "The token mint for token_out_2."
          ]
        },
        {
          "name": "userTokenInAccount",
          "docs": [
            "User's token_in account (source of payment)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "user"
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
          "name": "userTokenOut1Account",
          "docs": [
            "User's token_out_1 account (destination of token_out_1)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "user"
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
                "path": "tokenOutMint1"
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
          "name": "userTokenOut2Account",
          "docs": [
            "User's token_out_2 account (destination of token_out_2)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "user"
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
                "path": "tokenOutMint2"
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
            "Optional mint authority PDA for direct burning (when program has mint authority)"
          ],
          "optional": true,
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
          "name": "bossTokenInAccount",
          "docs": [
            "Boss's token_in account (destination of payment when no mint authority)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "boss"
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
          "name": "vaultTokenInAccount",
          "docs": [
            "Optional vault token_in account (for burning when program has mint authority)."
          ],
          "writable": true,
          "optional": true,
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
          "name": "vaultTokenOut1Account",
          "docs": [
            "Vault's token_out_1 account (source of token_out_1 to give)."
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
                "path": "tokenOutMint1"
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
          "name": "vaultTokenOut2Account",
          "docs": [
            "Vault's token_out_2 account (source of token_out_2 to give)."
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
                "path": "tokenOutMint2"
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
          "name": "user",
          "docs": [
            "The user taking the offer."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "SPL Token program."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
      "name": "takeSingleRedemptionOffer",
      "docs": [
        "Takes a single redemption offer.",
        "",
        "Delegates to `redemption_offer::take_single_redemption_offer`.",
        "Allows a user to exchange token_in for token_out based on the offer's price.",
        "Price is stored with token_in_decimals precision. Anyone can take the offer.",
        "Emits a `TakeSingleRedemptionOfferEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `TakeSingleRedemptionOffer`.",
        "- `offer_id`: ID of the offer to take.",
        "- `token_in_amount`: Amount of token_in to provide."
      ],
      "discriminator": [
        82,
        115,
        245,
        224,
        93,
        92,
        243,
        78
      ],
      "accounts": [
        {
          "name": "singleRedemptionOfferAccount",
          "docs": [
            "The single redemption offer account containing all offers."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  105,
                  110,
                  103,
                  108,
                  101,
                  95,
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
                  115
                ]
              }
            ]
          }
        },
        {
          "name": "state",
          "docs": [
            "Program state to get the boss."
          ]
        },
        {
          "name": "boss",
          "docs": [
            "The boss account that receives token_in payments.",
            "This must match the boss in the state account."
          ]
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "The single redemption vault authority that controls vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  105,
                  110,
                  103,
                  108,
                  101,
                  95,
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
                  118,
                  97,
                  117,
                  108,
                  116,
                  95,
                  97,
                  117,
                  116,
                  104
                ]
              }
            ]
          }
        },
        {
          "name": "tokenInMint",
          "docs": [
            "The token mint for token_in.",
            "Must be mutable to allow burning when program has mint authority"
          ],
          "writable": true
        },
        {
          "name": "tokenOutMint",
          "docs": [
            "The token mint for token_out."
          ]
        },
        {
          "name": "userTokenInAccount",
          "docs": [
            "User's token_in account (source of payment)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "user"
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
            "User's token_out account (destination of tokens)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "user"
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
          "name": "mintAuthorityPda",
          "docs": [
            "Optional mint authority PDA for direct burning (when program has mint authority)"
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
          "name": "bossTokenInAccount",
          "docs": [
            "Boss's token_in account (destination of payment when no mint authority)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "boss"
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
          "name": "vaultTokenInAccount",
          "docs": [
            "Optional vault token_in account (for burning when program has mint authority)."
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
            "Vault's token_out account (source of tokens to give)."
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
          "name": "user",
          "docs": [
            "The user taking the offer."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "SPL Token program."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": []
    },
    {
      "name": "updateBuyOfferFee",
      "docs": [
        "Updates the fee basis points for a buy offer.",
        "",
        "Delegates to `buy_offer::update_buy_offer_fee`.",
        "Allows the boss to modify the fee charged when users take the buy offer.",
        "Emits a `BuyOfferFeeUpdatedEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `UpdateBuyOfferFee`.",
        "- `offer_id`: ID of the buy offer to update.",
        "- `new_fee_basis_points`: New fee in basis points (0-10000)."
      ],
      "discriminator": [
        252,
        143,
        152,
        180,
        66,
        151,
        46,
        138
      ],
      "accounts": [
        {
          "name": "buyOfferAccount",
          "docs": [
            "The buy offer account containing all buy offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  95,
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
    },
    {
      "name": "updateDualRedemptionOfferFee",
      "docs": [
        "Updates the fee basis points for a dual redemption offer.",
        "",
        "Delegates to `redemption_offer::update_dual_redemption_offer_fee`.",
        "Allows the boss to modify the fee charged when users take the dual redemption offer.",
        "Emits a `DualRedemptionOfferFeeUpdatedEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `UpdateDualRedemptionOfferFee`.",
        "- `offer_id`: ID of the dual redemption offer to update.",
        "- `new_fee_basis_points`: New fee in basis points (0-10000)."
      ],
      "discriminator": [
        210,
        142,
        144,
        12,
        75,
        143,
        89,
        244
      ],
      "accounts": [
        {
          "name": "dualRedemptionOfferAccount",
          "docs": [
            "The dual redemption offer account containing all dual redemption offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  117,
                  97,
                  108,
                  95,
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
    },
    {
      "name": "updateSingleRedemptionOfferFee",
      "docs": [
        "Updates the fee basis points for a single redemption offer.",
        "",
        "Delegates to `redemption_offer::update_single_redemption_offer_fee`.",
        "Allows the boss to modify the fee charged when users take the single redemption offer.",
        "Emits a `SingleRedemptionOfferFeeUpdatedEvent` upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `UpdateSingleRedemptionOfferFee`.",
        "- `offer_id`: ID of the single redemption offer to update.",
        "- `new_fee_basis_points`: New fee in basis points (0-10000)."
      ],
      "discriminator": [
        23,
        223,
        126,
        204,
        144,
        69,
        122,
        125
      ],
      "accounts": [
        {
          "name": "singleRedemptionOfferAccount",
          "docs": [
            "The single redemption offer account containing all single redemption offers"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  105,
                  110,
                  103,
                  108,
                  101,
                  95,
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
      "name": "buyOfferAccount",
      "discriminator": [
        133,
        58,
        184,
        29,
        30,
        246,
        204,
        136
      ]
    },
    {
      "name": "buyOfferVaultAuthority",
      "discriminator": [
        70,
        15,
        14,
        126,
        177,
        79,
        47,
        20
      ]
    },
    {
      "name": "dualRedemptionOfferAccount",
      "discriminator": [
        3,
        137,
        109,
        2,
        212,
        4,
        144,
        224
      ]
    },
    {
      "name": "dualRedemptionVaultAuthority",
      "discriminator": [
        174,
        183,
        201,
        125,
        54,
        44,
        191,
        19
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
      "name": "singleRedemptionOfferAccount",
      "discriminator": [
        147,
        253,
        140,
        13,
        219,
        99,
        149,
        241
      ]
    },
    {
      "name": "singleRedemptionVaultAuthority",
      "discriminator": [
        141,
        85,
        209,
        193,
        148,
        147,
        166,
        106
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
      "name": "buyOfferFeeUpdatedEvent",
      "discriminator": [
        252,
        37,
        39,
        183,
        62,
        76,
        251,
        131
      ]
    },
    {
      "name": "buyOfferMadeEvent",
      "discriminator": [
        76,
        250,
        64,
        147,
        6,
        201,
        35,
        56
      ]
    },
    {
      "name": "buyOfferVaultDepositEvent",
      "discriminator": [
        94,
        41,
        43,
        100,
        208,
        163,
        218,
        163
      ]
    },
    {
      "name": "buyOfferVaultWithdrawEvent",
      "discriminator": [
        249,
        21,
        100,
        179,
        132,
        168,
        118,
        215
      ]
    },
    {
      "name": "buyOfferVectorAddedEvent",
      "discriminator": [
        40,
        116,
        229,
        220,
        106,
        39,
        19,
        188
      ]
    },
    {
      "name": "buyOfferVectorDeletedEvent",
      "discriminator": [
        117,
        149,
        77,
        139,
        75,
        124,
        144,
        75
      ]
    },
    {
      "name": "closeBuyOfferEvent",
      "discriminator": [
        55,
        231,
        182,
        155,
        92,
        109,
        208,
        111
      ]
    },
    {
      "name": "closeDualRedemptionOfferEvent",
      "discriminator": [
        202,
        17,
        179,
        245,
        187,
        36,
        245,
        79
      ]
    },
    {
      "name": "closeSingleRedemptionOfferEvent",
      "discriminator": [
        119,
        19,
        54,
        13,
        63,
        141,
        52,
        62
      ]
    },
    {
      "name": "dualRedemptionOfferFeeUpdatedEvent",
      "discriminator": [
        215,
        203,
        186,
        157,
        229,
        205,
        67,
        46
      ]
    },
    {
      "name": "dualRedemptionOfferMadeEvent",
      "discriminator": [
        197,
        72,
        62,
        30,
        67,
        78,
        44,
        238
      ]
    },
    {
      "name": "dualRedemptionVaultDepositEvent",
      "discriminator": [
        210,
        173,
        133,
        188,
        191,
        29,
        53,
        181
      ]
    },
    {
      "name": "dualRedemptionVaultWithdrawEvent",
      "discriminator": [
        56,
        146,
        166,
        48,
        99,
        30,
        207,
        113
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
      "name": "singleRedemptionOfferFeeUpdatedEvent",
      "discriminator": [
        78,
        134,
        217,
        186,
        10,
        26,
        233,
        139
      ]
    },
    {
      "name": "singleRedemptionOfferMadeEvent",
      "discriminator": [
        153,
        203,
        254,
        191,
        172,
        20,
        189,
        216
      ]
    },
    {
      "name": "singleRedemptionVaultDepositEvent",
      "discriminator": [
        35,
        57,
        167,
        176,
        232,
        185,
        230,
        235
      ]
    },
    {
      "name": "singleRedemptionVaultWithdrawEvent",
      "discriminator": [
        42,
        33,
        100,
        219,
        229,
        49,
        36,
        176
      ]
    },
    {
      "name": "takeBuyOfferEvent",
      "discriminator": [
        59,
        147,
        67,
        41,
        42,
        119,
        163,
        88
      ]
    },
    {
      "name": "takeBuyOfferPermissionlessEvent",
      "discriminator": [
        64,
        186,
        164,
        220,
        143,
        172,
        74,
        44
      ]
    },
    {
      "name": "takeDualRedemptionOfferEvent",
      "discriminator": [
        114,
        131,
        40,
        206,
        32,
        226,
        157,
        247
      ]
    },
    {
      "name": "takeSingleRedemptionOfferEvent",
      "discriminator": [
        245,
        73,
        102,
        195,
        165,
        221,
        142,
        150
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
      "name": "buyOffer",
      "docs": [
        "Buy offer struct for token exchange with dynamic pricing"
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
                    "name": "buyOfferVector"
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
      "name": "buyOfferAccount",
      "docs": [
        "Account holding MAX_BUY_OFFERS BuyOffer instances"
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
                    "name": "buyOffer"
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
      "name": "buyOfferFeeUpdatedEvent",
      "docs": [
        "Event emitted when a buy offer's fee is updated."
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
      "name": "buyOfferMadeEvent",
      "docs": [
        "Event emitted when a buy offer is created."
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
      "name": "buyOfferVaultAuthority",
      "type": {
        "kind": "struct",
        "fields": []
      }
    },
    {
      "name": "buyOfferVaultDepositEvent",
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
      "name": "buyOfferVaultWithdrawEvent",
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
      "name": "buyOfferVector",
      "docs": [
        "Time vector for buy offers with pricing information"
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
              "APR represents the annualized rate of return for this buy offer.",
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
      "name": "buyOfferVectorAddedEvent",
      "docs": [
        "Event emitted when a time vector is added to a buy offer."
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
      "name": "buyOfferVectorDeletedEvent",
      "docs": [
        "Event emitted when a time vector is deleted from a buy offer."
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
      "name": "closeBuyOfferEvent",
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
      "name": "closeDualRedemptionOfferEvent",
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
      "name": "closeSingleRedemptionOfferEvent",
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
      "name": "dualRedemptionOffer",
      "docs": [
        "Dual redemption offer struct for token exchange with static pricing for two output tokens"
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
            "name": "tokenOutMint1",
            "type": "pubkey"
          },
          {
            "name": "tokenOutMint2",
            "type": "pubkey"
          },
          {
            "name": "startTime",
            "type": "u64"
          },
          {
            "name": "endTime",
            "type": "u64"
          },
          {
            "name": "price1",
            "type": "u64"
          },
          {
            "name": "price2",
            "type": "u64"
          },
          {
            "name": "ratioBasisPoints",
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
      "name": "dualRedemptionOfferAccount",
      "docs": [
        "Account holding MAX_DUAL_REDEMPTION_OFFERS DualRedemptionOffer instances"
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
                    "name": "dualRedemptionOffer"
                  }
                },
                50
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
      "name": "dualRedemptionOfferFeeUpdatedEvent",
      "docs": [
        "Event emitted when a dual redemption offer's fee is updated."
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
      "name": "dualRedemptionOfferMadeEvent",
      "docs": [
        "Event emitted when a dual redemption offer is created."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "startTime",
            "type": "u64"
          },
          {
            "name": "endTime",
            "type": "u64"
          },
          {
            "name": "price1",
            "type": "u64"
          },
          {
            "name": "price2",
            "type": "u64"
          },
          {
            "name": "ratioBasisPoints",
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
      "name": "dualRedemptionVaultAuthority",
      "type": {
        "kind": "struct",
        "fields": []
      }
    },
    {
      "name": "dualRedemptionVaultDepositEvent",
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
      "name": "dualRedemptionVaultWithdrawEvent",
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
      "name": "singleRedemptionOffer",
      "docs": [
        "Redemption offer struct for token exchange with static pricing"
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
            "name": "startTime",
            "type": "u64"
          },
          {
            "name": "endTime",
            "type": "u64"
          },
          {
            "name": "price",
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
      "name": "singleRedemptionOfferAccount",
      "docs": [
        "Account holding MAX_REDEMPTION_OFFERS SingleRedemptionOffer instances"
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
                    "name": "singleRedemptionOffer"
                  }
                },
                50
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
      "name": "singleRedemptionOfferFeeUpdatedEvent",
      "docs": [
        "Event emitted when a single redemption offer's fee is updated."
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
      "name": "singleRedemptionOfferMadeEvent",
      "docs": [
        "Event emitted when a single redemption offer is created."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "startTime",
            "type": "u64"
          },
          {
            "name": "endTime",
            "type": "u64"
          },
          {
            "name": "price",
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
      "name": "singleRedemptionVaultAuthority",
      "type": {
        "kind": "struct",
        "fields": []
      }
    },
    {
      "name": "singleRedemptionVaultDepositEvent",
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
      "name": "singleRedemptionVaultWithdrawEvent",
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
      "name": "takeBuyOfferEvent",
      "docs": [
        "Event emitted when a buy offer is successfully taken"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "docs": [
              "The ID of the buy offer that was taken"
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
      "name": "takeBuyOfferPermissionlessEvent",
      "docs": [
        "Event emitted when a buy offer is successfully taken via permissionless flow"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "docs": [
              "The ID of the buy offer that was taken"
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
    },
    {
      "name": "takeDualRedemptionOfferEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "tokenInAmount",
            "type": "u64"
          },
          {
            "name": "tokenOut1Amount",
            "type": "u64"
          },
          {
            "name": "tokenOut2Amount",
            "type": "u64"
          },
          {
            "name": "feeAmount",
            "type": "u64"
          },
          {
            "name": "user",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "takeSingleRedemptionOfferEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "tokenInAmount",
            "type": "u64"
          },
          {
            "name": "tokenOutAmount",
            "type": "u64"
          },
          {
            "name": "feeAmount",
            "type": "u64"
          },
          {
            "name": "user",
            "type": "pubkey"
          }
        ]
      }
    }
  ]
};
