#!/bin/bash
# Build (or reuse) the elohim-cache-core WASM module for elohim-app.
#
# elohim-cache-core is the protocol's browser-side caching layer, compiled to
# WASM via `wasm-pack build --target web`. The generated pkg/ output
# (elohim/elohim-cache-core/pkg/) is gitignored (pkg/.gitignore: `*`) — every
# consumer either builds it locally or gets a pre-built copy some other way:
#   - CI (root Jenkinsfile "Fetch WASM Cache Core" stage): fetches a
#     pre-built artifact from Harbor, published by the DNA pipeline
#     (elohim/holochain/dna/Jenkinsfile), which runs wasm-pack in a container
#     that has the toolchain. CI's "Build App" stage calls `ng build`
#     directly and never invokes `pnpm run build`/this script.
#   - Local dev / agent containers: this script, via the elohim-app prebuild
#     chain (`pnpm run build` / `pnpm start`).
#
# wasm-pack is not always installed in dev containers. Rather than hard-fail
# the whole prebuild chain (which also skips every OTHER prebuild step —
# fonts, sophia assets, service worker — producing confusing unrelated
# warnings downstream), degrade gracefully: if wasm-pack is missing but a
# previously built pkg/ output already exists on disk, reuse it and warn.
# elohim-app's runtime already tolerates a stale/missing WASM bundle (falls
# back to the pure TypeScript mirror — see
# elohim/elohim-cache-core/CLAUDE.md "TypeScript Mirror"), so reusing a
# slightly stale local build is safe. It is NOT safe to silently skip
# forever, so this still fails loud when there is nothing to reuse.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"               # app/elohim-app (script lives in app/elohim-app/scripts/)
PROJECT_ROOT="$(cd "$APP_DIR/../.." && pwd)"     # repo root
CACHE_CORE_DIR="$PROJECT_ROOT/elohim/elohim-cache-core"
PKG_DIR="$CACHE_CORE_DIR/pkg"
PKG_JS="$PKG_DIR/elohim_cache_core.js"
PKG_WASM="$PKG_DIR/elohim_cache_core_bg.wasm"


# @angular/build's application builder rejects asset inputs outside the workspace
# root (app/elohim-app), so angular.json copies the WASM from
# node_modules/elohim-cache-core rather than ../../elohim/elohim-cache-core/pkg.
# Mirror pkg/ into that location so a freshly built pkg/ is what ng build ships.
# (CI does the equivalent copy in the root Jenkinsfile after pnpm install.)
sync_to_node_modules() {
  local dest="$APP_DIR/node_modules/elohim-cache-core"
  mkdir -p "$dest"
  cp -f "$PKG_DIR"/*.js "$PKG_DIR"/*.wasm "$PKG_DIR"/*.d.ts "$PKG_DIR"/package.json "$dest/" 2>/dev/null || true
}

if command -v wasm-pack &>/dev/null; then
  (cd "$CACHE_CORE_DIR" && RUSTFLAGS='' wasm-pack build --target web --release)
  sync_to_node_modules
  exit 0
fi

if [ -f "$PKG_JS" ] && [ -f "$PKG_WASM" ]; then
  echo "wasm-pack not installed — reusing existing elohim-cache-core pkg output"
  sync_to_node_modules
  exit 0
fi

cat >&2 <<'EOF'
ERROR: wasm-pack not installed, and no existing elohim-cache-core/pkg build to reuse.

elohim-cache-core (the browser WASM caching layer) needs at least one real
wasm-pack build to seed pkg/. Install wasm-pack, then re-run:

  Prebuilt release binary (fast, no cargo install needed):
    curl -sL https://github.com/wasm-bindgen/wasm-pack/releases/latest/download/wasm-pack-v0.15.0-x86_64-unknown-linux-musl.tar.gz \
      | tar xz --strip-components=1 -C ~/bin '*/wasm-pack'
    (check https://github.com/wasm-bindgen/wasm-pack/releases/latest for the
    current version/target triple if this one 404s)

  Or via cargo:
    cargo install wasm-pack

Then re-run this build: pnpm run build:wasm   (from app/elohim-app)
EOF
exit 1
