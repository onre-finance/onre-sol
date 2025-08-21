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
      "name": "initialize",
      "docs": [
        "Creates an offer with two buy tokens.",
        "",
        "Delegates to `make_offer::make_offer_two`.",
        "The price of the sell token changes over time based on `sell_token_start_amount`,",
        "`sell_token_end_amount`, and `price_fix_duration` within the offer's active time window.",
        "Emits an `OfferMadeTwo` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `MakeOfferTwo`.",
        "- `offer_id`: Unique ID for the offer.",
        "- `buy_token_1_total_amount`: Total amount of the first buy token offered.",
        "- `buy_token_2_total_amount`: Total amount of the second buy token offered.",
        "- `sell_token_start_amount`: Sell token amount at the start of the offer.",
        "- `sell_token_end_amount`: Sell token amount at the end of the offer.",
        "- `offer_start_time`: Offer activation timestamp.",
        "- `offer_end_time`: Offer expiration timestamp.",
        "- `price_fix_duration`: Duration of each price interval.",
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
          "name": "vaultAuthority",
          "docs": [
            "The vault authority account to initialize, rent paid by `boss`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
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
      "name": "makeBuyOffer",
      "docs": [
        "Creates a buy offer.",
        "",
        "Delegates to `buy_offer::make_buy_offer`.",
        "The price of the token_out changes over time based on `start_price`,",
        "`end_price`, and `price_fix_duration` within the offer's active time window.",
        "Emits a `BuyOfferMade` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `MakeBuyOffer`.",
        "- `offer_id`: Unique ID for the offer."
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
          "name": "tokenInMint",
          "docs": [
            "Mint of the token_in for the offer."
          ]
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
      "name": "makeSingleRedemptionOffer",
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
            "The buy offer account within the BuyOfferAccount, rent paid by `boss`. Already initialized in initialize."
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
          "name": "tokenInMint",
          "docs": [
            "Mint of the token_in for the offer."
          ]
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
      "name": "takeSingleRedemptionOffer",
      "docs": [
        "Takes a single redemption offer.",
        "",
        "Delegates to `redemption_offer::take_single_redemption_offer`.",
        "Allows a user to exchange token_in for token_out based on the offer's price.",
        "Price is stored with 9 decimal precision. Anyone can take the offer.",
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
            "The vault authority that controls vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
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
            "The token mint for token_in."
          ]
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
          "name": "bossTokenInAccount",
          "docs": [
            "Boss's token_in account (destination of payment)."
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
      "name": "vaultDeposit",
      "docs": [
        "Deposits tokens into the vault.",
        "",
        "Delegates to `vault_operations::vault_deposit`.",
        "Transfers tokens from boss's account to vault's token account for the specified mint.",
        "Creates vault token account if it doesn't exist using init_if_needed.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `VaultDeposit`.",
        "- `amount`: Amount of tokens to deposit."
      ],
      "discriminator": [
        231,
        150,
        41,
        113,
        180,
        104,
        162,
        120
      ],
      "accounts": [
        {
          "name": "vaultAuthority",
          "docs": [
            "The vault authority account that controls the vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
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
      "name": "vaultWithdraw",
      "docs": [
        "Withdraws tokens from the vault.",
        "",
        "Delegates to `vault_operations::vault_withdraw`.",
        "Transfers tokens from vault's token account to boss's token account for the specified mint.",
        "Both token accounts must already exist.",
        "Only the boss can call this instruction.",
        "",
        "# Arguments",
        "- `ctx`: Context for `VaultWithdraw`.",
        "- `amount`: Amount of tokens to withdraw."
      ],
      "discriminator": [
        98,
        28,
        187,
        98,
        87,
        69,
        46,
        64
      ],
      "accounts": [
        {
          "name": "vaultAuthority",
          "docs": [
            "The vault authority account that controls the vault token accounts."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
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
            "Boss's token account for the specific mint (destination of tokens).",
            "Must already exist - will fail if it doesn't."
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
            "Vault's token account for the specific mint (source of tokens).",
            "Must already exist - will fail if it doesn't."
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
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    }
  ],
  "accounts": [
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
      "name": "vaultAuthority",
      "discriminator": [
        132,
        34,
        187,
        202,
        202,
        195,
        211,
        53
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
    },
    {
      "name": "vaultDepositEvent",
      "discriminator": [
        187,
        186,
        196,
        189,
        175,
        44,
        10,
        64
      ]
    },
    {
      "name": "vaultWithdrawEvent",
      "discriminator": [
        192,
        143,
        53,
        201,
        67,
        20,
        212,
        195
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "invalidBossAddress"
    }
  ],
  "types": [
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
            "name": "timeSegments",
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "buyOfferTimeSegment"
                  }
                },
                10
              ]
            }
          }
        ]
      }
    },
    {
      "name": "buyOfferAccount",
      "docs": [
        "Account holding MAX_BUY_OFFERS BuyOffer instances (should fit 10KB limit)"
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
            "name": "boss",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "buyOfferTimeSegment",
      "docs": [
        "Time segment for buy offers with pricing information"
      ],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "segmentId",
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
            "name": "startPrice",
            "type": "u64"
          },
          {
            "name": "endPrice",
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
          }
        ]
      }
    },
    {
      "name": "singleRedemptionOfferAccount",
      "docs": [
        "Account holding MAX_BUY_OFFERS RedemptionOfferSingle instances (should fit 10KB limit)"
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
      "name": "singleRedemptionOfferMadeEvent",
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
            "name": "user",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "vaultAuthority",
      "type": {
        "kind": "struct",
        "fields": []
      }
    },
    {
      "name": "vaultDepositEvent",
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
      "name": "vaultWithdrawEvent",
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
    }
  ]
};
