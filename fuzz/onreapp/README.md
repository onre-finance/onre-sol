# onreapp fuzz harness

Stateful invariant fuzzing for `programs/onreapp`, run on FuzzCorp by
`.github/workflows/fuzzcorp-submit.yml`. The workflow builds the program from source on every
push and fuzzes that build — nothing here ships a prebuilt `.so`.

## Layout

    src/main.rs        actions (one per instruction) + invariants
    build-bundle.sh    assembles the FuzzCorp bundle
    idls/              IDL the action bindings were generated from
    programs/          program artifacts, staged by CI (gitignored)

## Local run

```bash
cargo build-sbf --tools-version v1.54 --manifest-path ../../programs/onreapp/Cargo.toml -- --locked
mkdir -p programs && cp ../../target/deploy/onreapp.so programs/
cargo test --release          # deterministic regression tests
./build-bundle.sh             # bundle for FuzzCorp
```

`--tools-version v1.54` is deliberate: v1.51's bundled cargo predates edition2024 support, which
some current dependencies require; v1.52 emitted larger stack frames that pushed one instruction
past the SBF 4096-byte frame limit (the build still exits 0, and the instruction then fails at
runtime with an access violation that looks like a missing account). v1.54 has neither issue. CI
pins the same version and fails the build if the frame warning appears. `-- --locked` pins the
build to the committed `Cargo.lock` so a dependency published after this harness was last built
can't silently change what gets deployed.

## Invariants

See `PROPERTIES.md` for the full ledger (10 properties, P-0001 through P-0010) — what each one
claims, why it's a net rather than a mirror of an existing check, and its current status against
this program version.

All armed properties are asserted every run. Those already reported to the maintainers are
silenced by id via `SCOUT_CHECK_MUTE` in the bundle manifest, so campaigns surface only new
signal; muting is announced on stderr as `[SCOUT_CHECK_MUTED]`. Drop an id from that list once its
bug is fixed and the property becomes a regression guard.

## CI secrets

`FUZZ_API_KEY` (the only secret; crucible is public, so no token is needed to fetch it). Org
and project are repo variables `FUZZ_ORG` / `FUZZ_PROJECT`.
