# Fuzz tests for OnRe app

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