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
# Only one bootstrap/worker may own this state, including across restarts of the entrypoint.
exec 9>"$state/worker.lock"
flock -n 9 || refuse 'another worker owns this provider state'
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
  env -i HOME="$state/private-home" TMPDIR="$state/tmp" PATH=/usr/local/bin:/usr/bin:/bin \
    COMPUTE_IMAGE_ROOT="$image" COMPUTE_STATE_ROOT="$state" \
    MESH_CONTINUE_INITIALIZATION_LOG_SHA256="${COMPUTE_CONTINUE_INITIALIZATION_LOG_SHA256:-}" \
    MESH_CONTINUE_INITIALIZATION_STATE_SHA256="${COMPUTE_CONTINUE_INITIALIZATION_STATE_SHA256:-}" \
    bash "$here/mesh-env.sh" bash "$mesh_script" "$@" 9>&-
}
mesh_run start
mesh_run wait --timeout 900
reports="$repo/genesis/a2o/reports"
[[ ! -L "$reports" ]] || refuse 'reports cannot be a symlink'
# Only the report subtree and the declared runtime-config files are guest writable.
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
  chmod 606 "$path"
done
if [[ -f "$state/mesh/household-fixture.json" ]]; then
  chmod 644 "$state/mesh/household-fixture.json"
fi
printf '%s\n' "$commit" > "$state/mesh/.provider-mesh-ready"
chmod 644 "$state/mesh/.provider-mesh-ready"
export COMPUTE_WORKER_ROOT="$state/worker"
echo "provider bootstrap ready: repoRoot=$repo justBin=$image/mesh-just.sh"
cd "$repo"
node "$image/grant-bootstrap.mjs"
exec node "$image/worker.mjs"
