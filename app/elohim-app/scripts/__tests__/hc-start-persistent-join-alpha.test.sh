#!/usr/bin/env bash
# Join-alpha identity lifecycle classification. Does not launch a conductor.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/../hc-start.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Load only the pure classifier from hc-start.sh; sourcing the whole lifecycle
# script would start services and is intentionally outside this test's scope.
awk '
  /^hc_start_join_alpha_sandbox\(\)/ { capture=1 }
  capture { print }
  capture && /^}$/ { exit }
' "$script" > "$tmp/classifier.sh"
source "$tmp/classifier.sh"

awk '
  /^hc_start_resolve_doorway_happ\(\)/ { capture=1 }
  capture { print }
  capture && /^}$/ { exit }
' "$script" > "$tmp/doorway-bundle.sh"
source "$tmp/doorway-bundle.sh"

printf 'bundle\n' > "$tmp/local.happ"
printf 'bundle\n' > "$tmp/deployed.happ"
printf 'bundle\n' > "$tmp/override.happ"
[[ "$(hc_start_resolve_doorway_happ isolated "$tmp/local.happ" "$tmp/deployed.happ" "")" == "$tmp/local.happ" ]] || {
  echo "FAIL isolated doorway did not keep its local bundle" >&2; exit 1;
}
[[ "$(hc_start_resolve_doorway_happ join-alpha "$tmp/local.happ" "$tmp/deployed.happ" "")" == "$tmp/deployed.happ" ]] || {
  echo "FAIL join-alpha doorway did not default to the deployed bundle" >&2; exit 1;
}
[[ "$(hc_start_resolve_doorway_happ join-alpha "$tmp/local.happ" "$tmp/deployed.happ" "$tmp/override.happ")" == "$tmp/override.happ" ]] || {
  echo "FAIL explicit HAPP_BUNDLE_PATH did not select the doorway bundle" >&2; exit 1;
}
if out="$(hc_start_resolve_doorway_happ join-alpha "$tmp/local.happ" "$tmp/deployed.happ" "$tmp/missing.happ" 2>&1)"; then
  echo "FAIL missing explicit doorway bundle was accepted" >&2; exit 1;
fi
[[ "$out" == *"HAPP_BUNDLE_PATH"* ]] || { echo "FAIL missing bundle refusal did not name the setting" >&2; exit 1; }
echo "ok   doorway bundle defaults and explicit override are isolated from conductor hApp selection"

# Exercise the exact argument array consumed by both maintained storage launch
# branches; a candidate containing spaces must remain one argument.
awk '
  /^hc_start_storage_happ_args\(\)/ { capture=1 }
  capture { print }
  capture && /^}$/ { exit }
' "$script" > "$tmp/storage-bundle.sh"
source "$tmp/storage-bundle.sh"
cp "$tmp/override.happ" "$tmp/persistent candidate.happ"
hc_start_storage_happ_args "$tmp/persistent candidate.happ"
[[ "${#STORAGE_HAPP_ARGS[@]}" == 2 && "${STORAGE_HAPP_ARGS[0]}" == --happ-path && "${STORAGE_HAPP_ARGS[1]}" == "$tmp/persistent candidate.happ" ]]
hc_start_storage_happ_args ""
[[ "${#STORAGE_HAPP_ARGS[@]}" == 0 ]]
if out="$(hc_start_storage_happ_args "$tmp/missing.happ" 2>&1)"; then
  echo "FAIL missing storage bundle accepted" >&2; exit 1
fi
[[ "$out" == *STORAGE_HAPP_PATH* ]]
[[ "$(grep -Fc '"$STORAGE_BIN" --http-port "$STORAGE_PORT" "${STORAGE_HAPP_ARGS[@]}" &' "$script")" == 2 ]]
validation_line="$(grep -n '^hc_start_storage_happ_args "${STORAGE_HAPP_PATH:-}"' "$script" | cut -d: -f1)"
stop_line="$(grep -n '^if \[ "${1:-}" = "--stop"' "$script" | cut -d: -f1)"
[[ "$validation_line" -gt "$stop_line" ]]
pool_line="$(grep -n '^source "$REPO_ROOT/genesis/agentic/bin/pool-lib.sh"' "$script" | cut -d: -f1)"
[[ "$validation_line" -lt "$pool_line" ]]
echo "ok   storage candidate validated before launch (stop remains usable) and passed intact on both launch branches"

root="$tmp/local-dev"
mkdir -p "$root"

if out="$(hc_start_join_alpha_sandbox "$root" t3-join-alpha 0 2>&1)"; then
  echo "FAIL missing identity enrolled without explicit authorization: $out" >&2
  exit 1
fi
[[ "$out" == *"CONDUCTOR_ENROLL=1"* ]] || { echo "FAIL refusal did not name enrollment flag" >&2; exit 1; }
echo "ok   missing identity requires explicit enrollment"

[[ "$(hc_start_join_alpha_sandbox "$root" t3-join-alpha 1)" == generate ]] || {
  echo "FAIL explicit enrollment did not select generation" >&2; exit 1;
}
echo "ok   explicit enrollment selects first-time generation"

# Once any target directory exists, enrollment cannot turn a partial identity
# into permission to replace it.
mkdir -p "$root/t3-join-alpha/ks"
if out="$(hc_start_join_alpha_sandbox "$root" t3-join-alpha 1 2>&1)"; then
  echo "FAIL partial identity was accepted: $out" >&2
  exit 1
fi
[[ "$out" == *"refusing to overwrite"* ]] || { echo "FAIL partial-state refusal was unclear" >&2; exit 1; }
echo "ok   partial identity refuses even with enrollment flag"

mkdir -p "$root/household-peer" "$root/t3-join-alpha/ks" "$root/t3-join-alpha/databases"
printf '%s\n%s\n' "$root/household-peer" "$root/t3-join-alpha" > "$root/.hc"
printf 'config\n' > "$root/t3-join-alpha/conductor-config.yaml"
printf 'key-bytes\n' > "$root/t3-join-alpha/ks/store_file"
printf 'db-bytes\n' > "$root/t3-join-alpha/databases/conductor.db"
before="$(sha256sum "$root/t3-join-alpha/ks/store_file" | cut -d' ' -f1)"
[[ "$(hc_start_join_alpha_sandbox "$root" t3-join-alpha 0)" == 'resume|1' ]] || {
  echo "FAIL complete state did not select its exact registry index" >&2; exit 1;
}
after="$(sha256sum "$root/t3-join-alpha/ks/store_file" | cut -d' ' -f1)"
[[ "$before" == "$after" ]] || { echo "FAIL resume changed identity key bytes" >&2; exit 1; }
echo "ok   complete registered state resumes by exact index and preserves key bytes"

printf '%s\n%s\n' "$root/t3-join-alpha" "$root/t3-join-alpha" > "$root/.hc"
if out="$(hc_start_join_alpha_sandbox "$root" t3-join-alpha 1 2>&1)"; then
  echo "FAIL duplicate registry entries were accepted: $out" >&2
  exit 1
fi
[[ "$out" == *"inconsistently registered"* ]] || { echo "FAIL duplicate-state refusal was unclear" >&2; exit 1; }
echo "ok   duplicate registration refuses without overwrite"

# The script's alpha generation branch is guarded by the classifier; isolated
# mode continues using the original generate invocation.
grep -Fq 'hc_start_join_alpha_sandbox "$LOCAL_DEV_DIR" "$T3_SANDBOX_NAME" "${CONDUCTOR_ENROLL:-0}"' "$script"
grep -Fq 'exec hc sandbox generate --app-id elohim --in-process-lair $T3_SANDBOX_FLAGS -r=$CONDUCTOR_APP_PORT "$HAPP_PATH"' "$script"
grep -Fq 'exec hc sandbox run $JOIN_SANDBOX_INDEX' "$script"
echo "ok   join-alpha generation/resume and isolated generation launch branches are present"

grep -Fq 'export HOLOCHAIN_APP_URL="ws://localhost:$CONDUCTOR_APP_PORT"' "$script"
grep -Fq -- '--conductor-url "ws://localhost:$CONDUCTOR_APP_PORT"' "$script"
grep -Fq -- '--app-port-min "$CONDUCTOR_APP_PORT"' "$script"
grep -Fq -- '--app-port-max "$CONDUCTOR_APP_PORT"' "$script"
grep -Fq '"HAPP_BUNDLE_PATH=$DOORWAY_HAPP_PATH"' "$script"
grep -Fq 'DOORWAY_HAPP_PATH="$(hc_start_resolve_doorway_happ "$NETWORK_PROFILE" "$HAPP_PATH" "$JOIN_HAPP_PATH" "${HAPP_BUNDLE_PATH:-}")"' "$script"
grep -Fq 'AGENT_SDK_PORT="${ELOHIM_AGENT_PORT:-8096}"' "$script"
echo "ok   storage, doorway workers, and SDK use distinct T3 ports"
