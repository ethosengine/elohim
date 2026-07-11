#!/usr/bin/env bash
# Publish the eprfs library crate set to the internal Nexus registry (`elohim`).
# Mirrors scripts/ci/cargo-publish-epr.sh (single-crate) and brit's
# cargo-publish-brit.sh (multi-crate, topological), for the eprfs workspace.
#
# Scope: the 5 LIBRARY crates only. The eprfs-agent binary
# (elohim/sdk/domains/elohim-agent/adapter) is `publish = false` — it is baked
# into the dev/ci image build like brit-verify, never consumed as a registry
# crate, so its path deps stay path-only and it is excluded here.
#
# Idempotent + 409-safe: a crate+version already on the index is a no-op
# success, so this runs on every dev/main build and only publishes on a
# version bump. The eprfs crates share one workspace version (`[workspace.package]`),
# so they bump in lockstep.
#
# --no-verify: crates publish in dependency order, but the sparse index needs a
# beat to serve a just-published crate before a dependent could resolve it.
# --no-verify skips the redundant per-crate verify build (the workspace is
# already built+tested upstream) and sidesteps the index-propagation race.
#
# Auth: bind the CI Nexus npm token to NEXUS_NPM_TOKEN (cargo-deployer role on
# cargo-internal) and this derives the cargo Bearer header. Override: set
# CARGO_REGISTRIES_ELOHIM_TOKEN directly (e.g. a User Token as `Basic <b64>`).
set -euo pipefail

if [ -z "${CARGO_REGISTRIES_ELOHIM_TOKEN:-}" ] && [ -n "${NEXUS_NPM_TOKEN:-}" ]; then
  export CARGO_REGISTRIES_ELOHIM_TOKEN="Bearer ${NEXUS_NPM_TOKEN}"
fi

REPO_ROOT="${WORKSPACE:-$(git rev-parse --show-toplevel)}"
EPRFS_DIR="${REPO_ROOT}/elohim/eprfs"
WORKSPACE_MANIFEST="${EPRFS_DIR}/Cargo.toml"
INDEX_BASE="https://nexus.ethosengine.com/repository/cargo-internal"

# All eprfs library crates inherit `version.workspace = true`, so the single
# workspace version governs the whole set.
VERSION="$(awk '/^\[workspace.package\]/{p=1} p&&/^version[[:space:]]*=/{match($0,/"[^"]+"/); print substr($0,RSTART+1,RLENGTH-2); exit}' "${WORKSPACE_MANIFEST}")"
[ -n "${VERSION}" ] || { echo "ERROR: could not parse workspace version from ${WORKSPACE_MANIFEST}" >&2; exit 1; }
echo "eprfs crate-set target version: ${VERSION}"

# Topological order: dependencies before dependents.
#   eprfs-core, eprfs-host  (leaves)
#   eprfs-storage, eprfs-meta  (-> core)
#   eprfs-local  (-> core, host; dev-dep storage)
CRATES=(eprfs-core eprfs-host eprfs-storage eprfs-meta eprfs-local)

# Native crates: clear the WASM getrandom flag and any sccache wrapper.
export RUSTFLAGS=""
export RUSTC_WRAPPER=""
export CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS="cargo:token"

# Cargo sparse-index path rules, by crate-name length (all eprfs-* are >= 4).
sparse_path() {
  local n="$1"
  case "${#n}" in
    1) echo "1/${n}" ;;
    2) echo "2/${n}" ;;
    3) echo "3/${n:0:1}/${n}" ;;
    *) echo "${n:0:2}/${n:2:2}/${n}" ;;
  esac
}

cd "${EPRFS_DIR}"

for crate in "${CRATES[@]}"; do
  index_url="${INDEX_BASE}/$(sparse_path "${crate}")"

  if curl -fsSL "${index_url}" 2>/dev/null | grep -q "\"vers\":\"${VERSION}\""; then
    echo "${crate} ${VERSION} already on the index — nothing to publish."
    continue
  fi

  if [ -z "${CARGO_REGISTRIES_ELOHIM_TOKEN:-}" ]; then
    echo "ERROR: no cargo registry credential — bind ee-nexus-npm-token to NEXUS_NPM_TOKEN" >&2
    echo "       (or set CARGO_REGISTRIES_ELOHIM_TOKEN directly)." >&2
    exit 1
  fi

  echo "Publishing ${crate} ${VERSION} to the elohim registry…"
  set +e
  OUT="$(cargo publish -p "${crate}" --registry elohim --no-verify 2>&1)"
  RC=$?
  set -e
  echo "${OUT}"

  if [ ${RC} -ne 0 ]; then
    if echo "${OUT}" | grep -qiE 'already (exists|uploaded)|409|conflict'; then
      echo "${crate} ${VERSION} already present (409) — treating as success."
      continue
    fi
    exit ${RC}
  fi
  echo "Published ${crate} ${VERSION}."

  # Give the sparse index a beat to serve the new crate before its dependents.
  sleep 3
done

echo "eprfs publish stage complete."
