#!/usr/bin/env bash
# Build-only (Node/Rust capable container), never the bash-only deploy container.
# Source mode builds both native tools in their immutable runtime userspace:
#   build-compute-worker.sh --source BASE@sha256:DIGEST OUTPUT_TAG [COMMIT]
# Legacy packaging remains: BASE@sha256:DIGEST EXECUTOR ARK OUTPUT_TAG.
# This script builds a local image only. The owning CI stage publishes its digest.
set -euo pipefail

compute_source=0
if [[ "${1:-}" == --source ]]; then compute_source=1; shift; fi
compute_base=${1:?immutable compatible runtime image required}
[[ "$compute_base" =~ @sha256:[a-f0-9]{64}$ ]] || { echo 'Base image must be pinned by digest' >&2; exit 2; }
compute_context=$(mktemp -d)
trap 'rm -rf "$compute_context"' EXIT
compute_args=(--build-arg "WORKER_BASE_IMAGE=$compute_base")

if [[ "$compute_source" == 1 ]]; then
  compute_tag=${2:?output image tag required}
  compute_ref=${3:-HEAD}
  compute_repo=$(git rev-parse --show-toplevel)
  compute_commit=$(git -C "$compute_repo" rev-parse --verify --end-of-options "$compute_ref^{commit}")
  compute_rakia=$(git -C "$compute_repo" ls-tree "$compute_commit" -- elohim/rakia | awk '$1 == "160000" { print $3 }')
  compute_conductor=$(git -C "$compute_repo" ls-tree "$compute_commit" -- elohim/holochain-conductor | awk '$1 == "160000" { print $3 }')
  [[ "$compute_rakia" =~ ^[a-f0-9]{40}$ ]] || { echo 'Committed rakia gitlink required' >&2; exit 2; }
  [[ "$compute_conductor" =~ ^[a-f0-9]{40}$ ]] || { echo 'Committed conductor gitlink required' >&2; exit 2; }
  # A fresh shallow object store preserves SUT's git identity without copying local
  # credentials, hooks, remotes, index changes, untracked files, or worktree state.
  git -c init.templateDir= init -q "$compute_context/repo"
  git -C "$compute_context/repo" -c core.hooksPath=/dev/null fetch -q --depth=1 --no-tags --recurse-submodules=no "$compute_repo" "$compute_commit"
  git -C "$compute_context/repo" -c core.hooksPath=/dev/null checkout -q --detach FETCH_HEAD
  compute_snapshot_gitlink() {
    local compute_path=$1 compute_pin=$2 compute_source_url compute_local compute_snapshot
    compute_local="$compute_repo/$compute_path"
    compute_snapshot="$compute_context/repo/$compute_path"
    if [[ "$(git -C "$compute_local" rev-parse --show-toplevel 2>/dev/null || true)" == "$compute_local" ]] &&
        git -C "$compute_local" cat-file -e "$compute_pin^{commit}" 2>/dev/null; then
      compute_source_url=$compute_local
    else
      # CI intentionally leaves the conductor uninitialized (update=none).
      # Read the URL from the selected commit, never local config or worktree dirt.
      compute_source_url=$(git -C "$compute_repo" config --blob "$compute_commit:.gitmodules" --get "submodule.$compute_path.url") || {
        echo "Committed .gitmodules URL required for unavailable $compute_path pin" >&2; exit 2;
      }
      [[ -n "$compute_source_url" ]] || { echo "Empty committed URL for $compute_path" >&2; exit 2; }
    fi
    git -c init.templateDir= init -q "$compute_snapshot"
    git -C "$compute_snapshot" -c core.hooksPath=/dev/null fetch -q --depth=1 --no-tags --recurse-submodules=no -- "$compute_source_url" "$compute_pin"
    git -C "$compute_snapshot" -c core.hooksPath=/dev/null checkout -q --detach FETCH_HEAD
    [[ "$(git -C "$compute_snapshot" rev-parse HEAD)" == "$compute_pin" ]] || { echo "Fetched $compute_path differs from committed pin" >&2; exit 2; }
  }
  compute_snapshot_gitlink elohim/rakia "$compute_rakia"
  git -C "$compute_context/repo/elohim/rakia" cat-file -e HEAD:rakia-executor/Cargo.toml 2>/dev/null || {
    echo 'Pinned rakia does not contain the compute executor; commit it upstream and move the pin' >&2; exit 2;
  }
  compute_snapshot_gitlink elohim/holochain-conductor "$compute_conductor"
  git -C "$compute_context/repo/elohim/holochain-conductor" cat-file -e HEAD:crates/hc/Cargo.toml 2>/dev/null || {
    echo 'Pinned conductor does not contain the fork hc source' >&2; exit 2;
  }
  # Checkout otherwise stamps every source as "built now", falsely making a pinned
  # native asset look stale. Preserve a reproducible source time for mesh preflight.
  compute_epoch=$(git -C "$compute_context/repo" show -s --format=%ct "$compute_commit")
  compute_rakia_epoch=$(git -C "$compute_context/repo/elohim/rakia" show -s --format=%ct "$compute_rakia")
  compute_conductor_epoch=$(git -C "$compute_context/repo/elohim/holochain-conductor" show -s --format=%ct "$compute_conductor")
  (cd "$compute_context/repo" && git ls-files -z | xargs -0 -r touch -h -d "@$compute_epoch" --)
  (cd "$compute_context/repo/elohim/rakia" && git ls-files -z | xargs -0 -r touch -h -d "@$compute_rakia_epoch" --)
  (cd "$compute_context/repo/elohim/holochain-conductor" && git ls-files -z | xargs -0 -r touch -h -d "@$compute_conductor_epoch" --)
  # The mesh checks these two native source trees against the packaged binaries.
  # A later harness-only commit must not make unchanged native source look newer.
  # Read history from the original checkout: the sanitized snapshot is depth one.
  compute_shallow=$(git -C "$compute_repo" rev-parse --path-format=absolute --git-path shallow)
  compute_times="$compute_context/repo/.compute-source-times.json"
  [[ ! -e "$compute_times" ]] || { echo 'Reserved source timestamp receipt already exists' >&2; exit 2; }
  printf '{"sourceCommit":"%s","sources":[' "$compute_commit" > "$compute_times"
  compute_separator=''
  for compute_path in elohim/elohim-storage/src doorway/doorway-service/src; do
    compute_tree=$(git -C "$compute_repo" rev-parse --verify "$compute_commit:$compute_path")
    [[ "$(git -C "$compute_repo" cat-file -t "$compute_tree")" == tree ]] || { echo "Native source tree required: $compute_path" >&2; exit 2; }
    # Read both fields from the path-scoped log; git show can inspect a dirty
    # worktree .gitmodules even when asked only for commit metadata.
    compute_change=$(git -C "$compute_repo" log -1 --format='%H %ct' "$compute_commit" -- "$compute_path")
    read -r compute_last_change compute_native_epoch <<< "$compute_change"
    [[ "$compute_last_change" =~ ^[a-f0-9]{40}$ ]] || { echo "Native source history required: $compute_path" >&2; exit 2; }
    if [[ -f "$compute_shallow" ]] && grep -Fxq "$compute_last_change" "$compute_shallow"; then
      echo "REFUSED native source time: $compute_path reaches a shallow history boundary; deepen the checkout" >&2
      exit 2
    fi
    [[ "$(git -C "$compute_repo" rev-parse --verify "$compute_last_change:$compute_path")" == "$compute_tree" ]] || {
      echo "REFUSED native source time: last-change tree differs for $compute_path" >&2; exit 2;
    }
    [[ "$compute_native_epoch" =~ ^[0-9]+$ ]] || { echo "Invalid native source time: $compute_path" >&2; exit 2; }
    (cd "$compute_context/repo" && git ls-files -z -- "$compute_path" | xargs -0 -r touch -h -d "@$compute_native_epoch" --)
    printf '%s{"path":"%s","tree":"%s","lastChangeCommit":"%s","epoch":%s}' \
      "$compute_separator" "$compute_path" "$compute_tree" "$compute_last_change" "$compute_native_epoch" >> "$compute_times"
    compute_separator=','
  done
  printf ']}\n' >> "$compute_times"
  compute_dockerfile="$compute_context/repo/genesis/agentic/compute/Dockerfile.source"
  [[ -f "$compute_dockerfile" ]] || { echo 'Committed source worker Dockerfile required' >&2; exit 2; }
  compute_args+=(--build-arg "SOURCE_COMMIT=$compute_commit" --build-arg "RAKIA_COMMIT=$compute_rakia" --build-arg "CONDUCTOR_COMMIT=$compute_conductor")
  echo "compute worker source: $compute_commit rakia: $compute_rakia conductor: $compute_conductor"
else
  compute_executor=${2:?prebuilt compute-executor required}
  compute_ark=${3:?prebuilt ark required}
  compute_tag=${4:?output image tag required}
  cp "$compute_executor" "$compute_context/compute-executor"
  cp "$compute_ark" "$compute_context/ark"
  cp genesis/agentic/compute/{common,worker,workspace,payloads,retention,grant-bootstrap}.mjs "$compute_context/"
  cp genesis/agentic/compute/{worker-start,mesh-just,mesh-env}.sh "$compute_context/"
  cp genesis/agentic/compute/Dockerfile "$compute_context/Dockerfile"
  compute_dockerfile="$compute_context/Dockerfile"
fi

compute_builder=${COMPUTE_WORKER_BUILDER:-}
if [[ -z "$compute_builder" ]]; then
  if command -v nerdctl >/dev/null 2>&1; then compute_builder=nerdctl; else compute_builder=docker; fi
fi
case "$compute_builder" in
  nerdctl) compute_command=(nerdctl -n k8s.io build);;
  docker) compute_command=(docker build);;
  *) echo 'COMPUTE_WORKER_BUILDER must be nerdctl or docker' >&2; exit 2;;
esac
compute_command+=("${compute_args[@]}" --tag "$compute_tag" --file "$compute_dockerfile" "$compute_context")
if [[ "${COMPUTE_WORKER_BUILD_DRY_RUN:-0}" == 1 ]]; then
  printf 'compute worker build:'; printf ' %q' "${compute_command[@]}"; printf '\n'
  exit 0
fi
if [[ "$compute_builder" == nerdctl ]]; then
  export BUILDKIT_HOST=${BUILDKIT_HOST:-unix:///run/buildkit/buildkitd.sock}
  buildctl --addr "$BUILDKIT_HOST" debug workers >/dev/null
fi
"${compute_command[@]}"
