# Coverage

This repo is prepared for the `sbpf-coverage` flow and the command sequence below has been verified locally.

- <https://github.com/LimeChain/sbpf-coverage>

## What is wired

Rust LiteSVM tests can emit the trace artifacts that `sbpf-coverage` consumes.

In particular:

- `programs/onreapp/Cargo.toml` enables LiteSVM register tracing for Rust tests:

```toml
[dev-dependencies]
litesvm = { version = "0.10.0", features = ["precompiles", "register-tracing"] }
```

- the repo root `Cargo.toml` has:

```toml
[profile.release]
debug = true
```

## Verified locally

Verified on 2026-04-10 with the current Rust LiteSVM test harness.

Running tests with `SBF_TRACE_DIR` generates trace artifacts such as:

- `*.regs`
- `*.insns`
- `*.program_id`
- `*.exec.sha256`

inside `sbf_trace_dir/`.

The full suite command also works:

```bash
SBF_TRACE_DIR=$PWD/sbf_trace_dir cargo test -p onreapp --tests -- --nocapture
```

## External prerequisites

Install:

1. `sbpf-coverage`
2. `lcov` / `genhtml`

Example:

```bash
cargo install sbpf-coverage
brew install lcov
```

## Verified Workflow

1. Build the program with debug info preserved.

For coverage-oriented builds, the `sbpf-coverage` README recommends low optimization and debug info, for example:

```bash
cargo build-sbf --debug --tools-version v1.52 --arch v1
```

2. Run Rust LiteSVM tests while collecting traces:

```bash
rm -rf sbf_trace_dir coverage
SBF_TRACE_DIR=$PWD/sbf_trace_dir cargo test -p onreapp --tests -- --nocapture
```

3. Generate coverage:

```bash
sbpf-coverage \
  --src-path=$PWD/programs/onreapp/src \
  --sbf-path=$PWD/target/deploy \
  --sbf-trace-dir=$PWD/sbf_trace_dir
```

This produces `*.lcov` files in `sbf_trace_dir/`.

4. Render HTML:

```bash
genhtml --output-directory coverage sbf_trace_dir/*.lcov --rc branch_coverage=1
open coverage/index.html
```

## Notes

- This is for the Rust LiteSVM test path, not `anchor test`.
- `SBF_TRACE_DISASSEMBLE=1` can also be set if you want `.trace` disassembly files in addition to coverage inputs.
- On this repo and toolchain, `sbpf-coverage` found debug symbols in `target/deploy/onreapp.debug`, so `--sbf-path=$PWD/target/deploy` is the correct path.
- `cargo build-sbf --arch v1` also generates `target/sbpfv1-solana-solana/`. That directory is build output only and should stay untracked.
- The generated directories `sbf_trace_dir/` and `coverage/` are build artifacts and should stay untracked.
- The most recent verified `genhtml` run completed successfully and produced `coverage/index.html`. That run reported overall line coverage of `53.8% (922/1713)`, so this document verifies the setup, not a high-coverage threshold.
