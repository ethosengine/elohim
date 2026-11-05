#!/usr/bin/env bash
# Publish the elohim-epr crate, and the addressing kernel it depends on
# (`elohim-address`, crates/elohim-address), to the internal Nexus registry (`elohim`).
# The kernel goes first: elohim-epr's registry manifest names it by version.
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
INDEX_BASE="https://nexus.ethosengine.com/repository/cargo-internal"

# Native crates: clear the WASM getrandom flag and any sccache wrapper.
export RUSTFLAGS=""
export RUSTC_WRAPPER=""
export CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS="cargo:token"

# publish_crate NAME MANIFEST_DIR PACKAGE_DIR
#   NAME          crate name (sparse-index path {c1}{c2}/{c3}{c4}/{name}, length >= 4)
#   MANIFEST_DIR  directory holding the crate's Cargo.toml (for its version)
#   PACKAGE_DIR   directory `cargo publish` runs from (its workspace root)
publish_crate() {
  local name="$1" manifest="$2/Cargo.toml" package_dir="$3"
  local version index_url out rc
  version="$(grep -m1 '^version = ' "${manifest}" | sed -E 's/.*"([^"]+)".*/\1/')"
  [ -n "${version}" ] || { echo "ERROR: could not parse version from ${manifest}" >&2; exit 1; }
  echo "${name} target version: ${version}"

  index_url="${INDEX_BASE}/${name:0:2}/${name:2:2}/${name}"
  if curl -fsSL "${index_url}" 2>/dev/null | grep -q "\"vers\":\"${version}\""; then
    echo "${name} ${version} already on the index — nothing to publish."
    return 0
  fi

  if [ -z "${CARGO_REGISTRIES_ELOHIM_TOKEN:-}" ]; then
    echo "ERROR: no cargo registry credential — bind ee-nexus-npm-token to NEXUS_NPM_TOKEN" >&2
    echo "       (or set CARGO_REGISTRIES_ELOHIM_TOKEN directly)." >&2
    exit 1
  fi

  set +e
  out="$(cd "${package_dir}" && cargo publish -p "${name}" --registry elohim 2>&1)"
  rc=$?
  set -e
  echo "${out}"

  if [ ${rc} -ne 0 ]; then
    # Lost a race or rebuilt the same version — treat "already published" as success.
    if echo "${out}" | grep -qiE 'already (exists|uploaded)|409|conflict'; then
      echo "${name} ${version} already present (409) — treating as success."
      return 0
    fi
    exit ${rc}
  fi
  echo "Published ${name} ${version} to the elohim registry."
}

publish_crate elohim-address "${REPO_ROOT}/crates/elohim-address" "${REPO_ROOT}/crates/elohim-address"
publish_crate elohim-epr "${REPO_ROOT}/elohim/epr" "${REPO_ROOT}/elohim"
