/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/onre_app.json`.
 */
export type OnreApp = {
  "address": "onreuGhHHgVzMWSkj2oQDLDtvvGvoepBPkqyaubFcwe",
  "metadata": {
    "name": "onreApp",
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
      "name": "closeOfferOne",
      "docs": [
        "Closes an offer with one buy token.",
        "",
        "Delegates to `close_offer::close_offer_one` to transfer remaining tokens and close the offer.",
        "Emits `TokensTransferred` and `OfferClosed` events."
      ],
      "discriminator": [
        247,
        43,
        30,
        202,
        34,
        58,
        184,
        52
      ],
      "accounts": [
        {
          "name": "offer",
          "docs": [
            "The offer account to be closed, with rent refunded to `boss`."
          ],
          "writable": true
        },
        {
          "name": "offerSellTokenAccount",
          "docs": [
            "Offer's sell token ATA, must exist prior to execution, controlled by `offer_token_authority`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "offer.sell_token_mint",
                "account": "offer"
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
          "name": "offerBuy1TokenAccount",
          "docs": [
            "Offer's buy token 1 ATA, must exist prior to execution, controlled by `offer_token_authority`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "offer.buy_token_1.mint",
                "account": "offer"
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
          "name": "bossBuy1TokenAccount",
          "docs": [
            "Boss's buy token 1 ATA, must exist prior to execution, owned by `boss`."
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
                "path": "offer.buy_token_1.mint",
                "account": "offer"
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
          "name": "bossSellTokenAccount",
          "docs": [
            "Boss's sell token ATA, must exist prior to execution, owned by `boss`."
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
                "path": "offer.sell_token_mint",
                "account": "offer"
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
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "offerTokenAuthority",
          "docs": [
            "Derived PDA for token authority, does not store data.",
            "",
            "# Note",
            "This account is marked with `CHECK` as it’s validated by the seed derivation."
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
              },
              {
                "kind": "account",
                "path": "offer.offer_id",
                "account": "offer"
              }
            ]
          }
        },
        {
          "name": "boss",
          "docs": [
            "The signer authorizing the closure, typically the boss."
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
            "SPL Token program for token operations."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account operations."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "closeOfferTwo",
      "docs": [
        "Closes an offer with two buy tokens.",
        "",
        "Delegates to `close_offer::close_offer_two` to transfer remaining tokens and close the offer.",
        "Emits `TokensTransferred` and `OfferClosed` events."
      ],
      "discriminator": [
        167,
        161,
        190,
        239,
        98,
        202,
        193,
        211
      ],
      "accounts": [
        {
          "name": "offer",
          "docs": [
            "The offer account to be closed, with rent refunded to `boss`."
          ],
          "writable": true
        },
        {
          "name": "offerSellTokenAccount",
          "docs": [
            "Offer's sell token ATA, must exist prior to execution, controlled by `offer_token_authority`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "offer.sell_token_mint",
                "account": "offer"
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
          "name": "offerBuy1TokenAccount",
          "docs": [
            "Offer's buy token 1 ATA, must exist prior to execution, controlled by `offer_token_authority`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "offer.buy_token_1.mint",
                "account": "offer"
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
          "name": "offerBuy2TokenAccount",
          "docs": [
            "Offer's buy token 2 ATA, must exist prior to execution, controlled by `offer_token_authority`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "offer.buy_token_2.mint",
                "account": "offer"
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
          "name": "bossBuy1TokenAccount",
          "docs": [
            "Boss's buy token 1 ATA, must exist prior to execution, owned by `boss`."
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
                "path": "offer.buy_token_1.mint",
                "account": "offer"
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
          "name": "bossBuy2TokenAccount",
          "docs": [
            "Boss's buy token 2 ATA, must exist prior to execution, owned by `boss`."
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
                "path": "offer.buy_token_2.mint",
                "account": "offer"
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
          "name": "bossSellTokenAccount",
          "docs": [
            "Boss's sell token ATA, must exist prior to execution, owned by `boss`."
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
                "path": "offer.sell_token_mint",
                "account": "offer"
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
          "name": "state",
          "docs": [
            "Program state, ensures `boss` is authorized."
          ]
        },
        {
          "name": "offerTokenAuthority",
          "docs": [
            "Derived PDA for token authority, does not store data.",
            "",
            "# Note",
            "This account is marked with `CHECK` as it’s validated by the seed derivation."
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
              },
              {
                "kind": "account",
                "path": "offer.offer_id",
                "account": "offer"
              }
            ]
          }
        },
        {
          "name": "boss",
          "docs": [
            "The signer authorizing the closure, typically the boss."
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
            "SPL Token program for token operations."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "docs": [
            "Solana System program for account operations."
          ],
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "getApy",
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
            "The offer to read NAV data from"
          ]
        }
      ],
      "args": [],
      "returns": "u64"
    },
    {
      "name": "getCurrentOffer",
      "docs": [
        "Returns the current offer ID"
      ],
      "discriminator": [
        225,
        35,
        70,
        115,
        7,
        112,
        171,
        191
      ],
      "accounts": [],
      "args": [],
      "returns": "u64"
    },
    {
      "name": "getNav",
      "docs": [
        "Returns the current NAV (price) for the given offer.",
        "NAV = current_sell_token_amount / buy_token_one_amount"
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
            "The offer to read NAV data from"
          ]
        }
      ],
      "args": [],
      "returns": "u64"
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
      "name": "makeOfferOne",
      "docs": [
        "Creates an offer with one buy token.",
        "",
        "Delegates to `make_offer::make_offer_one`.",
        "The price of the sell token changes over time based on `sell_token_start_amount`,",
        "`sell_token_end_amount`, and `price_fix_duration` within the offer's active time window.",
        "Emits an `OfferMadeOne` event upon success.",
        "",
        "# Arguments",
        "- `ctx`: Context for `MakeOfferOne`.",
        "- `offer_id`: Unique ID for the offer.",
        "- `buy_token_total_amount`: Total amount of the buy token offered.",
        "- `sell_token_start_amount`: Sell token amount at the start of the offer.",
        "- `sell_token_end_amount`: Sell token amount at the end of the offer.",
        "- `offer_start_time`: Offer activation timestamp.",
        "- `offer_end_time`: Offer expiration timestamp.",
        "- `price_fix_duration`: Duration of each price interval."
      ],
      "discriminator": [
        252,
        57,
        117,
        106,
        175,
        66,
        183,
        53
      ],
      "accounts": [
        {
          "name": "offer",
          "docs": [
            "The offer account to be initialized, with rent paid by `boss`.",
            "",
            "# Note",
            "- Space is allocated as `8 + Offer::INIT_SPACE` bytes, where 8 bytes are for the discriminator.",
            "- Seeded with `\"offer\"` and `offer_id` for PDA derivation."
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
                "kind": "arg",
                "path": "offerId"
              }
            ]
          }
        },
        {
          "name": "offerSellTokenAccount",
          "docs": [
            "Offer's sell token ATA, must exist prior to execution, controlled by `offer_token_authority`.",
            "",
            "# Note",
            "Included for future sell token transfers when the offer is taken."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "sellTokenMint"
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
          "name": "offerBuyToken1Account",
          "docs": [
            "Offer's buy token 1 ATA, must exist prior to execution, controlled by `offer_token_authority`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "buyToken1Mint"
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
          "name": "offerTokenAuthority",
          "docs": [
            "Derived PDA for token authority, does not store data.",
            "",
            "# Note",
            "This account is marked with `CHECK` as it's validated by the seed derivation."
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
              },
              {
                "kind": "arg",
                "path": "offerId"
              }
            ]
          }
        },
        {
          "name": "bossBuyToken1Account",
          "docs": [
            "Boss's buy token 1 ATA, must exist prior to execution, owned by `boss`."
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
                "path": "buyToken1Mint"
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
          "name": "sellTokenMint",
          "docs": [
            "Mint of the sell token for the offer."
          ]
        },
        {
          "name": "buyToken1Mint",
          "docs": [
            "Mint of the buy token 1 for the offer."
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
            "The signer funding and authorizing the offer creation, typically the boss."
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
            "SPL Token program for token operations."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
        },
        {
          "name": "buyTokenTotalAmount",
          "type": "u64"
        },
        {
          "name": "sellTokenStartAmount",
          "type": "u64"
        },
        {
          "name": "sellTokenEndAmount",
          "type": "u64"
        },
        {
          "name": "offerStartTime",
          "type": "u64"
        },
        {
          "name": "offerEndTime",
          "type": "u64"
        },
        {
          "name": "priceFixDuration",
          "type": "u64"
        }
      ]
    },
    {
      "name": "makeOfferTwo",
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
        "- `price_fix_duration`: Duration of each price interval."
      ],
      "discriminator": [
        213,
        55,
        86,
        231,
        52,
        147,
        202,
        4
      ],
      "accounts": [
        {
          "name": "offer",
          "docs": [
            "The offer account to be initialized, with rent paid by `boss`.",
            "",
            "# Note",
            "- Space is allocated as `8 + Offer::INIT_SPACE` bytes, where 8 bytes are for the discriminator.",
            "- Seeded with `\"offer\"` and `offer_id` for PDA derivation."
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
                "kind": "arg",
                "path": "offerId"
              }
            ]
          }
        },
        {
          "name": "offerSellTokenAccount",
          "docs": [
            "Offer's sell token ATA, must exist prior to execution, controlled by `offer_token_authority`.",
            "",
            "# Note",
            "Included for future sell token transfers when the offer is taken."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "sellTokenMint"
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
          "name": "offerBuyToken1Account",
          "docs": [
            "Offer's buy token 1 ATA, must exist prior to execution, controlled by `offer_token_authority`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "buyToken1Mint"
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
          "name": "offerBuyToken2Account",
          "docs": [
            "Offer's buy token 2 ATA, must exist prior to execution, controlled by `offer_token_authority`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "buyToken2Mint"
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
          "name": "offerTokenAuthority",
          "docs": [
            "Derived PDA for token authority, does not store data.",
            "",
            "# Note",
            "This account is marked with `CHECK` as it's validated by the seed derivation."
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
              },
              {
                "kind": "arg",
                "path": "offerId"
              }
            ]
          }
        },
        {
          "name": "bossBuyToken1Account",
          "docs": [
            "Boss's buy token 1 ATA, must exist prior to execution, owned by `boss`."
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
                "path": "buyToken1Mint"
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
          "name": "bossBuyToken2Account",
          "docs": [
            "Boss's buy token 2 ATA, must exist prior to execution, owned by `boss`."
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
                "path": "buyToken2Mint"
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
          "name": "sellTokenMint",
          "docs": [
            "Mint of the sell token for the offer."
          ]
        },
        {
          "name": "buyToken1Mint",
          "docs": [
            "Mint of the buy token 1 for the offer."
          ]
        },
        {
          "name": "buyToken2Mint",
          "docs": [
            "Mint of the buy token 2 for the offer."
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
            "The signer funding and authorizing the offer creation, typically the boss."
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
            "SPL Token program for token operations."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
        },
        {
          "name": "buyToken1TotalAmount",
          "type": "u64"
        },
        {
          "name": "buyToken2TotalAmount",
          "type": "u64"
        },
        {
          "name": "sellTokenStartAmount",
          "type": "u64"
        },
        {
          "name": "sellTokenEndAmount",
          "type": "u64"
        },
        {
          "name": "offerStartTime",
          "type": "u64"
        },
        {
          "name": "offerEndTime",
          "type": "u64"
        },
        {
          "name": "priceFixDuration",
          "type": "u64"
        }
      ]
    },
    {
      "name": "setBoss",
      "docs": [
        "Updates the boss in the program state.",
        "",
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
      "name": "takeOfferOne",
      "docs": [
        "Takes an offer with one buy token, respecting the current dynamic price.",
        "",
        "Delegates to `take_offer::take_offer_one`.",
        "The amount of buy token received is calculated based on the current price derived from the",
        "offer's dynamic pricing parameters.",
        "Emits an `OfferTakenOne` event.",
        "",
        "# Arguments",
        "- `ctx`: Context for `TakeOfferOne`.",
        "- `sell_token_amount`: Amount of sell tokens the user provides."
      ],
      "discriminator": [
        158,
        12,
        77,
        84,
        208,
        241,
        38,
        105
      ],
      "accounts": [
        {
          "name": "offer",
          "docs": [
            "The offer account being taken, providing offer details.",
            "Ensures this is a single buy token offer by checking `buy_token_mint_2`."
          ]
        },
        {
          "name": "offerSellTokenAccount",
          "docs": [
            "Offer's sell token ATA, receives the user's sell tokens."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "offer.sell_token_mint",
                "account": "offer"
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
          "name": "offerBuyToken1Account",
          "docs": [
            "Offer's buy token 1 ATA, sends buy tokens to the user."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "offer.buy_token_1.mint",
                "account": "offer"
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
          "name": "userSellTokenAccount",
          "docs": [
            "User's sell token ATA, sends sell tokens to the offer.",
            "Ensures mint matches the offer's sell token mint."
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
                "path": "offer.sell_token_mint",
                "account": "offer"
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
          "name": "userBuyToken1Account",
          "docs": [
            "User's buy token 1 ATA, receives buy tokens from the offer.",
            "Ensures mint matches the offer's buy token 1 mint."
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
                "path": "offer.buy_token_1.mint",
                "account": "offer"
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
          "name": "offerTokenAuthority",
          "docs": [
            "Derived PDA for token authority, controls offer token accounts.",
            "",
            "# Note",
            "This account is marked with `CHECK` as it's validated by the seed derivation."
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
              },
              {
                "kind": "account",
                "path": "offer.offer_id",
                "account": "offer"
              }
            ]
          }
        },
        {
          "name": "user",
          "docs": [
            "The user taking the offer, signs the transaction."
          ],
          "signer": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "SPL Token program for token operations."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
          "name": "sellTokenAmount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "takeOfferTwo",
      "docs": [
        "Takes an offer with two buy tokens, respecting the current dynamic price.",
        "",
        "Delegates to `take_offer::take_offer_two`.",
        "The amount of each buy token received is calculated based on the current price derived from the",
        "offer's dynamic pricing parameters.",
        "Emits an `OfferTakenTwo` event.",
        "",
        "# Arguments",
        "- `ctx`: Context for `TakeOfferTwo`.",
        "- `sell_token_amount`: Amount of sell tokens the user provides."
      ],
      "discriminator": [
        108,
        4,
        105,
        245,
        192,
        17,
        51,
        114
      ],
      "accounts": [
        {
          "name": "offer",
          "docs": [
            "The offer account being taken, providing offer details."
          ]
        },
        {
          "name": "offerSellTokenAccount",
          "docs": [
            "Offer's sell token ATA, receives the user's sell tokens."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "offer.sell_token_mint",
                "account": "offer"
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
          "name": "offerBuyToken1Account",
          "docs": [
            "Offer's buy token 1 ATA, sends buy token 1 to the user."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "offer.buy_token_1.mint",
                "account": "offer"
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
          "name": "offerBuyToken2Account",
          "docs": [
            "Offer's buy token 2 ATA, sends buy token 2 to the user."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "offerTokenAuthority"
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
                "path": "offer.buy_token_2.mint",
                "account": "offer"
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
          "name": "userSellTokenAccount",
          "docs": [
            "User's sell token account, sends sell tokens to the offer.",
            "Ensures mint matches the offer's sell token mint."
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
                "path": "offer.sell_token_mint",
                "account": "offer"
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
          "name": "userBuyToken1Account",
          "docs": [
            "User's buy token 1 ATA, receives buy token 1 from the offer.",
            "Ensures mint matches the offer's buy token 1 mint."
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
                "path": "offer.buy_token_1.mint",
                "account": "offer"
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
          "name": "userBuyToken2Account",
          "docs": [
            "User's buy token 2 ATA, receives buy token 2 from the offer.",
            "Ensures mint matches the offer's buy token 2 mint."
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
                "path": "offer.buy_token_2.mint",
                "account": "offer"
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
          "name": "offerTokenAuthority",
          "docs": [
            "Derived PDA for token authority, controls offer token accounts.",
            "",
            "# Note",
            "This account is marked with `CHECK` as it's validated by the seed derivation."
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
              },
              {
                "kind": "account",
                "path": "offer.offer_id",
                "account": "offer"
              }
            ]
          }
        },
        {
          "name": "user",
          "docs": [
            "The user taking the offer, signs the transaction."
          ],
          "signer": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "SPL Token program for token operations."
          ],
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
          "name": "sellTokenAmount",
          "type": "u64"
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
      "name": "offerClosed",
      "discriminator": [
        237,
        38,
        102,
        204,
        165,
        180,
        177,
        164
      ]
    },
    {
      "name": "offerMadeOne",
      "discriminator": [
        81,
        5,
        201,
        107,
        138,
        87,
        65,
        107
      ]
    },
    {
      "name": "offerMadeTwo",
      "discriminator": [
        205,
        102,
        99,
        79,
        132,
        214,
        101,
        36
      ]
    },
    {
      "name": "offerTakenOne",
      "discriminator": [
        236,
        152,
        189,
        93,
        219,
        176,
        60,
        184
      ]
    },
    {
      "name": "offerTakenTwo",
      "discriminator": [
        141,
        240,
        230,
        3,
        59,
        131,
        90,
        98
      ]
    },
    {
      "name": "tokensTransferred",
      "discriminator": [
        140,
        86,
        106,
        38,
        85,
        157,
        202,
        250
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "insufficientOfferTokenOneBalance",
      "msg": "Insufficient tokens remaining in the offer for token 1."
    },
    {
      "code": 6001,
      "name": "insufficientOfferTokenTwoBalance",
      "msg": "Insufficient tokens remaining in the offer for token 2."
    },
    {
      "code": 6002,
      "name": "invalidSellTokenMint",
      "msg": "The sell token mint does not match the offer."
    },
    {
      "code": 6003,
      "name": "invalidBuyTokenMint",
      "msg": "The buy token mint does not match the offer."
    },
    {
      "code": 6004,
      "name": "invalidTakeOffer",
      "msg": "The offer is of 2 buy token type."
    },
    {
      "code": 6005,
      "name": "calculationOverflow",
      "msg": "Calculation overflowed or invalid."
    },
    {
      "code": 6006,
      "name": "zeroBuyTokenAmount",
      "msg": "Zero buy token amount."
    },
    {
      "code": 6007,
      "name": "invalidCurrentTime",
      "msg": "Current time must be within the offer's start and end time range."
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
      "name": "offer",
      "docs": [
        "Represents an offer in the Onre App program.",
        "",
        "Stores details of an offer where a boss provides buy tokens in exchange for sell tokens.",
        "Used in `make_offer`, `take_offer`, and `close_offer` instructions.",
        "",
        "# Fields",
        "- `offer_id`: Unique identifier for the offer.",
        "- `sell_token_start_amount`: Initial amount of sell tokens required at offer start.",
        "- `sell_token_end_amount`: Final amount of sell tokens required at offer end.",
        "- `sell_token_mint`: Mint of the token the offer expects to receive.",
        "- `buy_token_1`: First buy token details (mint and amount).",
        "- `buy_token_2`: Second buy token details (mint and amount, defaults if unused).",
        "- `authority_bump`: Bump seed for the offer's token authority PDA.",
        "- `price_fix_duration`: Duration in seconds for each fixed price interval.",
        "- `offer_start_time`: Unix timestamp when the offer becomes active.",
        "- `offer_end_time`: Unix timestamp when the offer expires."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "sellTokenStartAmount",
            "type": "u64"
          },
          {
            "name": "sellTokenEndAmount",
            "type": "u64"
          },
          {
            "name": "sellTokenMint",
            "type": "pubkey"
          },
          {
            "name": "buyToken1",
            "type": {
              "defined": {
                "name": "offerToken"
              }
            }
          },
          {
            "name": "buyToken2",
            "type": {
              "defined": {
                "name": "offerToken"
              }
            }
          },
          {
            "name": "authorityBump",
            "type": "u8"
          },
          {
            "name": "priceFixDuration",
            "type": "u64"
          },
          {
            "name": "offerStartTime",
            "type": "u64"
          },
          {
            "name": "offerEndTime",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "offerClosed",
      "docs": [
        "Event emitted when an offer is closed."
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
          },
          {
            "name": "numBuyTokens",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "offerMadeOne",
      "docs": [
        "Event emitted when an offer with one buy token is created."
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
          },
          {
            "name": "buyTokenTotalAmount",
            "type": "u64"
          },
          {
            "name": "sellTokenStartAmount",
            "type": "u64"
          },
          {
            "name": "sellTokenEndAmount",
            "type": "u64"
          },
          {
            "name": "offerStartTime",
            "type": "u64"
          },
          {
            "name": "offerEndTime",
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
      "name": "offerMadeTwo",
      "docs": [
        "Event emitted when an offer with two buy tokens is created."
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
          },
          {
            "name": "buyToken1TotalAmount",
            "type": "u64"
          },
          {
            "name": "buyToken2TotalAmount",
            "type": "u64"
          },
          {
            "name": "sellTokenStartAmount",
            "type": "u64"
          },
          {
            "name": "sellTokenEndAmount",
            "type": "u64"
          },
          {
            "name": "offerStartTime",
            "type": "u64"
          },
          {
            "name": "offerEndTime",
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
      "name": "offerTakenOne",
      "docs": [
        "Event emitted when an offer with one buy token is taken."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "sellTokenAmount",
            "type": "u64"
          },
          {
            "name": "buyToken1Amount",
            "type": "u64"
          },
          {
            "name": "remainingBuyTokenAmount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "offerTakenTwo",
      "docs": [
        "Event emitted when an offer with two buy tokens is taken."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "sellTokenAmount",
            "type": "u64"
          },
          {
            "name": "buyToken1Amount",
            "type": "u64"
          },
          {
            "name": "buyToken2Amount",
            "type": "u64"
          },
          {
            "name": "remainingBuyToken1Amount",
            "type": "u64"
          },
          {
            "name": "remainingBuyToken2Amount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "offerToken",
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
      "name": "tokensTransferred",
      "docs": [
        "Event emitted when tokens are transferred during offer closure."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "offerId",
            "type": "u64"
          },
          {
            "name": "fromAccount",
            "type": "pubkey"
          },
          {
            "name": "toAccount",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          }
        ]
      }
    }
  ]
};
