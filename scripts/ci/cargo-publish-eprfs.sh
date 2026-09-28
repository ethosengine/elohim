#!/usr/bin/env bash
# Build-only entrypoint for the five eprfs library crates. The helper checks
# immutable registry receipts before treating an existing version as done.
set -euo pipefail

if [ -z "${CARGO_REGISTRIES_ELOHIM_TOKEN:-}" ] && [ -n "${NEXUS_NPM_TOKEN:-}" ]; then
  export CARGO_REGISTRIES_ELOHIM_TOKEN="Bearer ${NEXUS_NPM_TOKEN}"
fi

export RUSTFLAGS=""
export RUSTC_WRAPPER=""
export CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS="cargo:token"

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [ "$#" -gt 1 ]; then
  echo "usage: $0 [--check|--publish]" >&2
  exit 2
fi
exec python3 "${script_dir}/publish_eprfs.py" "${1:---publish}"
