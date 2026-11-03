# Dependency versions and compatibility

Versions were checked against crates.io, npm, and the official Anchor release
notes on 2026-11-03. Only stable direct dependency releases are selected.
The program/package version is unchanged in this dependency upgrade.

## Build and client tools

| Component | Selected version | Notes |
| --- | --- | --- |
| Anchor CLI, anchor-lang, anchor-spl, TypeScript client | 1.2.2 | Rust crates and client are pinned together. |
| Solana CLI | 4.1.2 | Anchor 1.2.2 recommended toolchain. |
| Solana platform-tools | v1.57 | Default in workspace metadata and Anchor 1.2.2. |
| SBPF | v3 | Normal builds and LiteSVM tests. |
| Host Rust | 1.99.1 | Stable; pinned by rust-toolchain.toml. |
| Node | 22.13 or later | New CLI dependencies require Node 22.13+. |
| pnpm | 12.16.1 | Pinned by packageManager; use this version for the lockfile. |
| TypeScript / Vite | 7.0.2 / 8.3.5 | Bundler module resolution and Vite asset types. |
| LiteSVM | 0.18.1 | Agave runtime is constrained to 4.3.2. |
| Trident fuzz crate / CLI | 0.12.0 | Latest stable; separate Solana 2.3 runtime. |

The TypeScript client moved from `@coral-xyz/anchor` to `@anchor-lang/core`.
Node ESM accesses `Wallet` through the package default export because Node
cannot discover that CommonJS re-export as a named import. Direct `bn.js`
imports now have an explicit package dependency. The CLI commands and
transaction flows remain the same. Anchor IDL generation also emits the
`target/types/onreapp_errors.ts` error-code/message helper; it is committed
alongside the existing generated types.

pnpm's `allowBuilds` explicitly permits the existing native binding packages
and esbuild to run their install scripts. Install with:

```bash
npm install --global pnpm@12.16.1
pnpm install --frozen-lockfile
avm install 1.2.2
```

AVM reads the project toolchain from Anchor.toml. Rustup reads the host Rust
version from rust-toolchain.toml. Anchor 1.2.2 selects platform-tools v1.57 and
SBPF v3 by default.

## Compatibility limits

| Dependency | Selected | Newer stable release checked | Reason |
| --- | --- | --- | --- |
| solana-program | 4.1.0 | 5.2.0 | 5.2 requires solana-epoch-schedule 3.4; LiteSVM 0.18.1 requires ~3.3.0. Cargo cannot resolve both. |
| solana-sdk | 4.1.0 | 5.0.0 | SDK 5 uses Account and Transaction major versions incompatible with LiteSVM 0.18.1. Verified by compiling the initialization test. |
| Agave test runtime | 4.3.2 | 4.4.0 | 4.4 changes runtime APIs and types used by LiteSVM. solana-builtins is pinned to keep the Agave crates aligned. |
| @solana/web3.js | 1.98.4 | 3.1.1 | Anchor 1.2.2 and @solana/spl-token 0.4.15 depend on web3.js 1.x. |

Other direct dependencies use the latest stable releases checked, including
SPL Token-2022 12.0.0, solana-instructions-sysvar 5.0.0,
solana-system-interface 3.3.0, solana-security-txt 1.1.3, cfg-if 1.0.5,
solana-compute-budget-interface 3.1.0, and Borsh 1.8.1 in the fuzz workspace.
The lockfiles preserve the tested transitive dependency set. Do not blindly
replace them with an unrestricted Cargo update: LiteSVM currently accepts
incompatible newer Agave releases in its declared version ranges.

## Validation and operational impact

Build the SBF artifact before running LiteSVM tests because the tests embed
`target/deploy/onreapp.so` at compile time:

```bash
anchor build
cargo test --locked -p onreapp
pnpm exec tsc --noEmit
pnpm ui:build
pnpm cli --help
cargo check --locked --manifest-path trident-tests/Cargo.toml
```

The program ID, instruction surface, account layouts, and application math
are intended to remain unchanged. Anchor 1.2.2 adds framework checks around
account closure/ownership and Token-2022 mint reuse; see its release notes.
Deployments must continue to support SIMD-0312 CreateAccountAllowPrefund.

Trident 0.12 uses an older Solana 2.3 runtime. Its system program does not
implement CreateAccountAllowPrefund, which the existing application uses.
The harness compiles, but a fuzz campaign needs runtime support for that
instruction; the dependency update does not establish that support.
LiteSVM 0.18.1 runs the program against the compatible Agave 4.3 runtime.

The Dockerfiles install the pinned Rust, Anchor, and Solana tools on top of
the existing published Anchor image because an Anchor 1.2.2 image is not
published. The Surfpool image also updates Node/pnpm, uses the frozen lockfile,
and lets pnpm install Vite's matching optional native binding instead of
forcing an old Rolldown binding.

## Upgrade verification

The v3 SBF build and all 545 Rust tests passed. The regenerated IDL is exactly
equal to the programV6 IDL. TypeScript checking, the UI production build, CLI
help startup, frozen pnpm installation, Rust formatting, and the Trident
harness compile check passed. Docker validation could not get past base-image
resolution in the local Docker daemon (`no match for platform in manifest`,
including an explicit linux/amd64 check); container builds were not validated.

## Sources

- [Anchor 1.2.2 release notes](https://www.anchor-lang.com/docs/updates/release-notes/1-2-2)
- [LiteSVM 0.18.1 crate and its published source/lockfile](https://crates.io/crates/litesvm/0.18.1)
- [Anchor TypeScript client](https://www.npmjs.com/package/@anchor-lang/core)
- [SPL Token client](https://www.npmjs.com/package/@solana/spl-token)
