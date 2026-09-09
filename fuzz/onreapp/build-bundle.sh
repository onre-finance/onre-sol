#!/usr/bin/env bash
# Assemble the FuzzCorp bundle for the onreapp harness.
#
# Expects the program artifacts to ALREADY be staged into ./programs by the caller (CI
# does this from a fresh `cargo build-sbf`). This script deliberately does not build them
# itself: the point of living in this repo is that every push recompiles the program and
# refuzzes it, so the build must be the repo's own rather than a copy that can drift.
#
#   ./build-bundle.sh                  assemble bundle/
#   ./build-bundle.sh --check          regenerate the manifest and diff the committed one
#   ./build-bundle.sh --write-manifest rewrite manifest.fc.json from targets.txt
set -euo pipefail

# Coverage gate: a symbols file with no DWARF still satisfies `[ -f ]` and still
# reports "not stripped" from `file`, then renders EMPTY coverage on the server
# while every CI step reports success. The server's complaint in that case is
# "SourcesOriginalPath ... does not match any source file", which reads like a path
# bug rather than a missing-debug-info bug. Require real compile units instead.
scout_has_dwarf() {
  so="$1"; dd=""
  command -v llvm-dwarfdump >/dev/null 2>&1 && dd=llvm-dwarfdump
  [ -z "$dd" ] && command -v dwarfdump >/dev/null 2>&1 && dd=dwarfdump
  if [ -z "$dd" ]; then
    echo "warning: no llvm-dwarfdump available; cannot verify DWARF in $so" >&2
    return 0
  fi
  n=$($dd --debug-info "$so" 2>/dev/null | grep -c DW_TAG_compile_unit)
  if [ "${n:-0}" -eq 0 ]; then
    echo "warning: $so carries NO DWARF (0 compile units) -- coverage would render" >&2
    echo "         empty. Build the program with CARGO_PROFILE_RELEASE_DEBUG=2 and" >&2
    echo "         CARGO_PROFILE_RELEASE_STRIP=none." >&2
    return 1
  fi
  echo "coverage: $(basename "$so") carries $n compile units"
  return 0
}

cd "$(dirname "$0")"

MODE="${1:-build}"
BUNDLE="${BUNDLE_DIR:-bundle}"
REPO_ROOT="$(cd ../.. && pwd)"
PROGRAM="onreapp"

# Workers are linux/amd64. Building anything else produces a bundle that uploads and
# validates cleanly and is then never picked up by a worker -- it fails as silence, not
# as an error. Never emulate the toolchain through QEMU; rustc segfaults under it.
TARGET="x86_64-unknown-linux-gnu"
ARCH="amd64"

# Invariants already reported to the maintainers. Muted so campaigns surface only NEW
# signal; drop an id once its bug is fixed and the property becomes a regression guard.
# `-` not `:-`: an explicitly empty value means "mute nothing" (used when validating that
# the harness still detects the known findings), whereas unset means "the reported set".
MUTE="${SCOUT_CHECK_MUTE-P-0002,P-0004,P-0005,P-0009}"

emit_manifest() {
  local commit="$1" out="$2" confs="" first=1
  while read -r kind crate feature; do
    case "$kind" in ''|\#*) continue ;; esac
    [ "$first" = 1 ] || confs+=","
    first=0
    confs+=$(cat <<CONF
{
      "Name": "${crate}__${feature}",
      "Confs": [
        {
          "Name": "explore",
          "Driver": {
            "Type": "crucible",
            "Params": {
              "BinaryPathInBundle": "bin/${crate}/${feature}",
              "HarnessRunDirInBundle": "bin/${crate}",
              "SymbolsPathInBundle": "symbols/${PROGRAM}.debug.so",
              "SourcesPathInBundle": "srcs",
              "SourcesOriginalPath": "programs/${PROGRAM}/src/",
              "ExtraEnv": { "SCOUT_CHECK_MUTE": "${MUTE}" }
            }
          },
          "Architecture": { "Name": "${ARCH}" },
          "MemoryKiB": 2097152,
          "Cores": 1,
          "StallTimeMinutes": 0,
          "YieldTimeMinutes": 120
        }
      ]
    }
CONF
)
  done < targets.txt

  cat > "$out" <<MANIFEST
{
  "Version": 3,
  "Revision": { "Commit": "${commit}" },
  "Lineages": [
    ${confs}
  ]
}
MANIFEST
}

if [ "$MODE" = "--check" ]; then
  tmp="$(mktemp)"; emit_manifest "PLACEHOLDER" "$tmp"
  ref="$(mktemp)"; sed 's/"Commit": "[^"]*"/"Commit": "PLACEHOLDER"/' manifest.fc.json > "$ref"
  if diff -u "$ref" "$tmp"; then echo "manifest.fc.json is in sync with targets.txt"; else
    echo "manifest.fc.json has drifted from targets.txt -- run ./build-bundle.sh --write-manifest" >&2
    exit 1
  fi
  exit 0
fi

COMMIT="$(git rev-parse HEAD 2>/dev/null || echo 0000000)"

if [ "$MODE" = "--write-manifest" ]; then
  emit_manifest "$COMMIT" manifest.fc.json
  echo "wrote manifest.fc.json"
  exit 0
fi

[ -f "programs/${PROGRAM}.so" ] || {
  echo "missing programs/${PROGRAM}.so -- build and stage the program first" >&2
  exit 1
}

rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/symbols" "$BUNDLE/srcs"

while read -r kind crate feature; do
  case "$kind" in ''|\#*) continue ;; esac
  echo "building $crate/$feature for $TARGET"
  # Native on an amd64 linux runner; cross-linked via zig off an arm64 dev box. Both
  # produce a real x86-64 ELF -- neither goes through emulation.
  if [ "$(uname -s)-$(uname -m)" = "Linux-x86_64" ]; then
    cargo build --release --features "$feature" --target "$TARGET"
  else
    cargo zigbuild --release --features "$feature" --target "$TARGET"
  fi
  mkdir -p "$BUNDLE/bin/$crate"
  cp "target/$TARGET/release/$feature" "$BUNDLE/bin/$crate/$feature"
  # The harness resolves its program as `programs/<program>.so` relative to its working
  # directory, which is what HarnessRunDirInBundle pins to bin/<crate>.
  cp -r programs idls "$BUNDLE/bin/$crate/"
done < targets.txt

file "$BUNDLE"/bin/*/* | grep -q "x86-64" || {
  echo "staged harness binary is not x86-64 -- workers would never schedule it" >&2
  exit 1
}

# Unstripped DWARF for source-level coverage. Staged under symbols/ and NOT under any
# path containing `target/`: crucible splits FUZZ_SYMBOLS at `/target/` to infer the
# source root, so a target/... path corrupts DWARF resolution to 0 source files.
if [ -f "programs/${PROGRAM}_symbols.so" ] && scout_has_dwarf "programs/${PROGRAM}_symbols.so"; then
  cp "programs/${PROGRAM}_symbols.so" "$BUNDLE/symbols/${PROGRAM}.debug.so"
else
  echo "note: no ${PROGRAM}_symbols.so; coverage will be bytecode-level" >&2
fi

# Sources for the coverage task's LCOV line mapping. `cargo build-sbf` records paths
# RELATIVE to the workspace root for this program -- verified by reading the emitted
# debug info, which carries `programs/onreapp/src/...` and no absolute prefix. So
# SourcesOriginalPath is that relative prefix and srcs/ mirrors the program's src/.
cp -r "$REPO_ROOT/programs/${PROGRAM}/src/." "$BUNDLE/srcs/"

emit_manifest "$COMMIT" "$BUNDLE/manifest.fc.json"
cp targets.txt "$BUNDLE/targets.txt"

echo "bundle staged at $BUNDLE"
find "$BUNDLE" -maxdepth 3 -not -path '*/.*' | sed 's/^/  /' | head -20
