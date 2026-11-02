#!/usr/bin/env bash
# Root bootstrap for an opt-in provider-owned household. Never resets mesh identities.
set -euo pipefail
here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
image="${COMPUTE_IMAGE_ROOT:-/opt/compute}"
if [[ "${COMPUTE_PRIVATE_MESH:-0}" != 1 ]]; then
  node "$image/grant-bootstrap.mjs"
  exec node "$image/worker.mjs"
fi
refuse() { echo "REFUSED provider bootstrap: $*" >&2; exit 2; }
[[ "$(id -u)" == 0 ]] || refuse 'private mesh bootstrap requires root; guests never control processes'
[[ "${MESH_RESET:-0}" == 0 ]] || refuse 'MESH_RESET is forbidden; retain provider identities'
state="${COMPUTE_STATE_ROOT:-/var/lib/compute}"
[[ "$state" == /* && "$state" != / && "$state" != /var && "$state" != /var/lib ]] || refuse 'use a dedicated absolute compute state root'
umask 077
mkdir -p "$state/repos" "$state/private-home" "$state/tmp"
chmod 755 "$state" "$state/repos"
chmod 700 "$state/private-home" "$state/tmp"
[[ "$(stat -c '%u:%g:%a' "$state/private-home")" == 0:0:700 ]] ||
  refuse 'private-home must be owned by root:root with mode 0700'
# Only one bootstrap/worker may own this state, including across restarts of the entrypoint.
exec 9>"$state/worker.lock"
flock -n 9 || refuse 'another worker owns this provider state'
load_or_create_secret() { # <root-only-file> <output-variable>
  local path="$1" output="$2" directory temporary metadata value
  directory="$(dirname -- "$path")"
  if [[ -e "$path" || -L "$path" ]]; then
    metadata="$(stat -c '%F:%u:%g:%a' -- "$path" 2>/dev/null)" ||
      refuse "cannot inspect provider secret $path"
    [[ "$metadata" == 'regular file:0:0:600' ]] ||
      refuse "provider secret must be a regular file owned by root:root with mode 0600: $path"
  else
    temporary="$(mktemp "$directory/.mesh-secret.XXXXXX")" ||
      refuse "cannot create provider secret beside $path"
    if ! head -c 32 /dev/urandom | base64 > "$temporary"; then
      rm -f -- "$temporary"
      refuse "cryptographic secret generation failed for $path"
    fi
    chmod 600 "$temporary"
    chown 0:0 "$temporary"
    mv -T -- "$temporary" "$path"
  fi
  IFS= read -r value < "$path" || refuse "provider secret is unreadable: $path"
  [[ ${#value} -ge 32 ]] || refuse "provider secret is shorter than 32 characters: $path"
  printf -v "$output" '%s' "$value"
}
# Both values are read only from each process's environment at launch (doorway config.rs:175/339,
# storage compute_tasks.rs:19); nothing persists them except storage's root-only restart capture,
# which `start` rewrites. A pod start launches every mesh process afresh, so a generated value
# takes effect with no migration and the repository-known fixture literals are retired.
load_or_create_secret "$state/private-home/mesh-api-key-admin" mesh_api_key_admin
load_or_create_secret "$state/private-home/mesh-compute-local-token" mesh_compute_local_token
for binary in elohim-storage doorway holochain hc iroh-relay relay-addr-beacon; do
  [[ -x "$image/mesh-bin/$binary" ]] || refuse "missing pinned image binary $binary"
done
[[ -x /usr/local/bin/mongod ]] || refuse 'missing pinned image mongod for the private archive'
[[ -r "$image/elohim.happ" ]] || refuse 'missing pinned image elohim.happ'
source_repo="$image/repo"
commit="$(git -C "$source_repo" rev-parse --verify HEAD)"
[[ "$commit" =~ ^[0-9a-f]{40,64}$ ]] || refuse 'image checkout has no commit identity'
[[ -z "${COMPUTE_SOURCE_COMMIT:-}" || "$COMPUTE_SOURCE_COMMIT" == "$commit" ]] || refuse 'image checkout differs from declared COMPUTE_SOURCE_COMMIT'
[[ -z "$(git -C "$source_repo" status --porcelain --untracked-files=no)" ]] || refuse 'image checkout has modified tracked source'
conductor_commit="$(git -C "$source_repo" rev-parse --verify HEAD:elohim/holochain-conductor)"
receipt="$image/mesh-bin/FORK_BUILD_RECEIPT.txt"
[[ -f "$receipt" ]] || refuse 'missing fork build receipt'
grep -Fxq "fork commit: $conductor_commit" "$receipt" || refuse 'fork build receipt differs from source gitlink'
for binary in holochain hc; do
  expected="$(awk -v path="/out/$binary" '$2 == path { print $1 }' "$receipt")"
  actual="$(sha256sum "$image/mesh-bin/$binary")"
  [[ "$expected" =~ ^[0-9a-f]{64}$ && "${actual%% *}" == "$expected" ]] || refuse "fork receipt digest differs for $binary"
done
repo="$state/repos/$commit"
if [[ ! -e "$repo" ]]; then
  staging="$(mktemp -d "$state/repos/.copy-$commit.XXXXXX")"
  cp -a "$source_repo/." "$staging/"
  printf '%s\n' "$commit" > "$staging/.compute-source-commit"
  chmod 644 "$staging/.compute-source-commit"
  chmod 755 "$staging"
  mv "$staging" "$repo"
fi
[[ -d "$repo" && ! -L "$repo" ]] || refuse 'pinned checkout must be a physical directory'
[[ "$(cat "$repo/.compute-source-commit")" == "$commit" ]] || refuse 'pinned checkout marker mismatch'
[[ -d "$repo/.git" && ! -L "$repo/.git" ]] || refuse 'pinned checkout has no git directory'
# A copy can lose directories that hold nothing: a detached shallow checkout keeps
# refs/heads and refs/tags empty, and git refuses a .git without refs/. Restore only
# the directories the image has and the copy lacks, with the image's modes. No file is
# written; the HEAD and clean-tree checks below still decide whether this is the pin.
while IFS= read -r -d '' rel; do
  [[ -e "$repo/.git/$rel" || -L "$repo/.git/$rel" ]] && continue
  mkdir -- "$repo/.git/$rel"
  chmod --reference="$source_repo/.git/$rel" -- "$repo/.git/$rel"
  echo "provider bootstrap: restored empty git directory .git/${rel#./} in the pinned checkout" >&2
done < <(cd "$source_repo/.git" && find . -mindepth 1 -type d -print0)
# Name the git directory, so a damaged checkout is never resolved through a parent.
pinned_git=(git --git-dir="$repo/.git" --work-tree="$repo")
head="$("${pinned_git[@]}" rev-parse --verify --end-of-options 'HEAD^{commit}' 2>&1)" ||
  refuse "pinned checkout is not a readable git checkout: ${head##*$'\n'}"
[[ "$head" == "$commit" ]] || refuse 'pinned checkout HEAD mismatch'
# A failing status prints nothing on stdout; read its exit before its silence.
tracked="$("${pinned_git[@]}" status --porcelain --untracked-files=no 2>&1)" ||
  refuse "pinned checkout status is unreadable: ${tracked##*$'\n'}"
[[ -z "$tracked" ]] || refuse 'pinned checkout has modified tracked source; refusing to overwrite it'
# Discovery pointer only. Stage profiles use repos/<commit> so an upgrade never retargets a task.
[[ ! -e "$state/repo" || -L "$state/repo" ]] || refuse 'legacy repo directory needs operator migration; will not overwrite it'
ln -sfn "repos/$commit" "$state/repo"
mesh_script="$repo/app/elohim-app/scripts/hc-mesh.sh"
[[ -f "$mesh_script" ]] || refuse 'pinned checkout has no mesh launcher'
echo "provider bootstrap: source $commit; persistent mesh $state/mesh; sequential worker admission"
# Do not leak the outer provider API token into captured mesh process environments.
mesh_run() {
  PROVIDER_MESH_ADMIN_SECRET="$mesh_api_key_admin" \
  PROVIDER_MESH_COMPUTE_SECRET="$mesh_compute_local_token" \
  PROVIDER_MESH_CONTINUE_LOG="${COMPUTE_CONTINUE_INITIALIZATION_LOG_SHA256:-}" \
  PROVIDER_MESH_CONTINUE_STATE="${COMPUTE_CONTINUE_INITIALIZATION_STATE_SHA256:-}" \
  python3 - "$state" "$image" "$here/mesh-env.sh" "$mesh_script" "$@" 9>&- <<'PY'
import os
import sys

state, image, mesh_env, mesh_script, *arguments = sys.argv[1:]
environment = {
    "HOME": f"{state}/private-home",
    "TMPDIR": f"{state}/tmp",
    "PATH": "/usr/local/bin:/usr/bin:/bin",
    "COMPUTE_IMAGE_ROOT": image,
    "COMPUTE_STATE_ROOT": state,
    "MESH_API_KEY_ADMIN": os.environ["PROVIDER_MESH_ADMIN_SECRET"],
    "MESH_COMPUTE_LOCAL_TOKEN": os.environ["PROVIDER_MESH_COMPUTE_SECRET"],
    "MESH_CONTINUE_INITIALIZATION_LOG_SHA256": os.environ["PROVIDER_MESH_CONTINUE_LOG"],
    "MESH_CONTINUE_INITIALIZATION_STATE_SHA256": os.environ["PROVIDER_MESH_CONTINUE_STATE"],
}
environment["MESH_DOORWAY_ENV_SET"] = "\n".join((
    f"API_KEY_ADMIN={environment['MESH_API_KEY_ADMIN']}",
    f"POOL_COMPUTE_TOKEN={environment['MESH_COMPUTE_LOCAL_TOKEN']}",
))
os.execve("/bin/bash", ["bash", mesh_env, "bash", mesh_script, *arguments], environment)
PY
}
mesh_run start
mesh_run wait --timeout 900
reports="$repo/genesis/a2o/reports"
[[ ! -L "$reports" ]] || refuse 'reports cannot be a symlink'
# The report subtree is the guest's only persistent write grant. Stage reports themselves are
# redirected to executor scratch, but `just test mesh` still emits its scoped config/summary here.
if [[ ! -d "$reports" ]]; then
  mkdir -p "$reports"
  chmod 755 "$reports"
  chown 65534:65534 "$reports"
fi
[[ "$(stat -c '%u' "$reports")" == 65534 ]] || refuse 'existing reports must be owned by the stage guest'
# Act as the report owner, so restart needs neither CAP_FOWNER nor CAP_DAC_OVERRIDE.
setpriv --reuid=65534 --regid=65534 --clear-groups bash -c '
  set -eu
  mkdir -p "$1/console"
  find "$1" -type d -exec chmod 755 {} +
  find "$1" -type f -exec chmod 644 {} +
' bash "$reports"
chmod 755 "$state/mesh"
for peer in matthew jessica james; do
  directory="$state/mesh/$peer"
  path="$directory/runtime-config.toml"
  [[ -f "$path" && ! -L "$path" ]] || refuse "missing owned runtime config for $peer"
  chmod 755 "$directory"
  chmod 644 "$path"
done
if [[ -f "$state/mesh/household-fixture.json" ]]; then
  chmod 644 "$state/mesh/household-fixture.json"
fi
printf '%s\n' "$commit" > "$state/mesh/.provider-mesh-ready"
chmod 644 "$state/mesh/.provider-mesh-ready"
export COMPUTE_WORKER_ROOT="$state/worker"
export COMPUTE_STATE_ROOT="$state"
export COMPUTE_SOURCE_COMMIT="$commit"
echo "provider bootstrap ready: repoRoot=$repo justBin=$image/mesh-just.sh"
cd "$repo"
# Issuing the grant on a freshly started conductor can time out: admission stays saturated for a
# while after a restart. Exiting would restart the container and the whole mesh, recreating that
# slow conductor, so a temporary refusal (exit 75: timeout, unreachable, 502/503/504) is retried
# in place. Issuance is content-addressed, so the identical request is safe to repeat. Bounded
# by COMPUTE_GRANT_RETRY_SECS (default 900); every other refusal ends the boot as before.
grant_deadline=$((SECONDS + ${COMPUTE_GRANT_RETRY_SECS:-900}))
until node "$image/grant-bootstrap.mjs"; do
  grant_status=$?
  if [[ "$grant_status" -ne 75 || "$SECONDS" -ge "$grant_deadline" ]]; then exit "$grant_status"; fi
  echo "provider grant bootstrap: temporary refusal; retrying the identical request in ${COMPUTE_GRANT_RETRY_INTERVAL_SECS:-30}s" >&2
  sleep "${COMPUTE_GRANT_RETRY_INTERVAL_SECS:-30}"
done
exec node "$image/worker.mjs"
