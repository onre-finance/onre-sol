# Fuzz tests for OnRe app

The V1 offer execution step, its boss-payment invariant, and the V1 instruction
bindings have been removed. `flow1` now exercises pricing-vector updates only.
V2 offer execution is covered by the LiteSVM suites in
`programs/onreapp/tests/take_offer.rs` and `take_offer_permissionless.rs`; this
Trident target does not currently fuzz V2 offer execution.

## Installation

Install Trident CLI tool

```bash
cargo install trident-cli@0.12.0
```

Verify that Trident is installed

```bash
trident --version
```


## Build the Anchor project

Build the anchor project as usual

```bash
anchor build
```

!!! note

    Trident.toml file contains path to the build anchor project, if by any chance the destination of the build binary is different from the default one, you need to update the path in the Trident.toml file.


## Run the fuzz tests

Run the fuzz tests
```bash
trident fuzz run fuzz_0
```


For more information about Trident, check the [documentation](https://ackee.xyz/trident/docs/dev/).

!!! note

    Fuzz tests are written using release candidate version of Trident, in documentation, at the time of writting this version is described in `dev` version and will be later released as version `0.12.0`.