#!/usr/bin/env bash
# Everything CI runs (.github/workflows/ci.yml), locally: lint, dead code (knip), the boundary
# between core and server, the modules of src/core/utils, the docs, the unit and server tests,
# both builds — the main one and the local one, checked for server parts (Paket L) — and, where
# cargo is found, the Rust modules: the crossover search and the optimizer with the shared
# vectors and the cases they were held to (decisions 0018, 0019).
#
#   npm run check            all of it
#   npm run check -- --quick without the Rust modules' tests in release mode (≈ 1 min)
set -euo pipefail
cd "$(dirname "$0")/.."

quick=0
[ "${1:-}" = --quick ] && quick=1

echo "== data (NTv2 grids)"; node tools/data.mjs
echo "== lint";  npm run -s lint
echo "== dead code"; npm run -s knip
echo "== unused translations"; npm run -s i18n:unused
echo "== boundary (core never imports from server)"; node tools/boundary.mjs --core
echo "== modules (through their index, no cycle, documented)"; node tools/modules.mjs
echo "== docs (links and file names)"; node tools/docs.mjs
echo "== tests"; npx vitest run
echo "== build (catches imports of names that are gone)"; npx vite build --outDir "$(mktemp -d)" --logLevel error
local_dist=$(mktemp -d)
echo "== local build (core alone, no server parts)"; npx vite build --mode standalone --outDir "$local_dist" --logLevel error
node tools/check-local-bundle.mjs "$local_dist"

# The Rust modules beside the page, where cargo is there (rustup).
cargo=${CARGO:-$(command -v cargo || echo "$HOME/.cargo/bin/cargo")}
if [ ! -x "$cargo" ]; then
  echo "== Rust: skipped (no cargo)"
  exit 0
fi
for crate in crossover optimizer; do
  echo "== Rust: $crate (format, clippy$([ "$quick" = 0 ] && echo ', tests'))"
  (cd "../../$crate" && "$cargo" fmt --check && "$cargo" clippy -q --all-targets -- -D warnings)
  if [ "$quick" = 0 ]; then (cd "../../$crate" && "$cargo" test -q --release >/dev/null); fi
done
