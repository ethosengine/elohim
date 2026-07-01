#!/usr/bin/env bash
# Publish the elohim-epr crate to the internal Nexus registry (`elohim`).
#
# Idempotent and 409-safe: if the version in Cargo.toml is already on the
# index, this is a no-op success — so the stage can run on every main build
# and only actually publishes when the version is bumped.
#
# Auth: a Nexus npm Bearer token authenticates cross-format, so we reuse the
# CI user's npm token — bind the Jenkins `ee-nexus-npm-token` secret to
# NEXUS_NPM_TOKEN and this derives the cargo header. (The CI user must hold
# repository-view add/read on cargo-internal — role `cargo-deployer`.)
# Override path: set CARGO_REGISTRIES_ELOHIM_TOKEN directly (e.g. a User Token
# as `Basic <base64>`) and it's used verbatim.
set -euo pipefail

if [ -z "${CARGO_REGISTRIES_ELOHIM_TOKEN:-}" ] && [ -n "${NEXUS_NPM_TOKEN:-}" ]; then
  export CARGO_REGISTRIES_ELOHIM_TOKEN="Bearer ${NEXUS_NPM_TOKEN}"
fi

REPO_ROOT="${WORKSPACE:-$(git rev-parse --show-toplevel)}"
MANIFEST="${REPO_ROOT}/elohim/epr/Cargo.toml"
INDEX_BASE="https://nexus.ethosengine.com/repository/cargo-internal"

VERSION="$(grep -m1 '^version = ' "${MANIFEST}" | sed -E 's/.*"([^"]+)".*/\1/')"
[ -n "${VERSION}" ] || { echo "ERROR: could not parse version from ${MANIFEST}" >&2; exit 1; }
echo "elohim-epr target version: ${VERSION}"

# Sparse-index path for a name of length >= 4: {c1}{c2}/{c3}{c4}/{name}
INDEX_URL="${INDEX_BASE}/el/oh/elohim-epr"
if curl -fsSL "${INDEX_URL}" 2>/dev/null | grep -q "\"vers\":\"${VERSION}\""; then
  echo "elohim-epr ${VERSION} already on the index — nothing to publish."
  exit 0
fi

if [ -z "${CARGO_REGISTRIES_ELOHIM_TOKEN:-}" ]; then
  echo "ERROR: no cargo registry credential — bind ee-nexus-npm-token to NEXUS_NPM_TOKEN" >&2
  echo "       (or set CARGO_REGISTRIES_ELOHIM_TOKEN directly)." >&2
  exit 1
fi

# Native crate: clear the WASM getrandom flag and any sccache wrapper.
export RUSTFLAGS=""
export RUSTC_WRAPPER=""
export CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS="cargo:token"

cd "${REPO_ROOT}/elohim"

set +e
OUT="$(cargo publish -p elohim-epr --registry elohim 2>&1)"
RC=$?
set -e
echo "${OUT}"

if [ ${RC} -ne 0 ]; then
  # Lost a race or rebuilt the same version — treat "already published" as success.
  if echo "${OUT}" | grep -qiE 'already (exists|uploaded)|409|conflict'; then
    echo "elohim-epr ${VERSION} already present (409) — treating as success."
    exit 0
  fi
  exit ${RC}
fi

echo "Published elohim-epr ${VERSION} to the elohim registry."
