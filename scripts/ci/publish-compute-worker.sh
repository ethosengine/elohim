#!/usr/bin/env bash
# Build-container only. Called after fleet images are published, under Harbor login.
# Produces an immutable worker pin and input provenance; activation is a separate
# reviewed renderer configuration. No fleet operation or grant issuance occurs here.
set -euo pipefail
compute_happ=${1:?path to the DNA pipeline elohim.happ required}
[[ -s "$compute_happ" ]] || { echo 'REFUSED compute image: hApp missing' >&2; exit 2; }
compute_root=$(git rev-parse --show-toplevel)
compute_commit=$(git rev-parse HEAD)
compute_base=${COMPUTE_WORKER_BASE_IMAGE:-harbor.ethosengine.com/devspaces/udi-plus-mem-rust-nix@sha256:0faed38daa8df52ec7b8c97b4ce310b0ee64c63d283d7c688c27759d3774c6ee}
compute_registry=harbor.ethosengine.com/ethosengine
compute_tools="$compute_registry/elohim-compute-worker:tools-$compute_commit"
compute_image="$compute_registry/elohim-compute-worker:$compute_commit"
compute_output=${COMPUTE_WORKER_IMAGE_REPORT_DIR:-$compute_root/compute-worker-image}
mkdir -p "$compute_output"
# Remove any stale success artifact before an attempt can refuse.
rm -f "$compute_output/image.json"
compute_context=$(mktemp -d)
trap 'rm -rf "$compute_context"' EXIT

immutable_image() {
  local ref=$1 repo=${1%:*} pins pin
  nerdctl -n k8s.io pull "$ref" >&2
  pins=$(nerdctl -n k8s.io image inspect "$ref" --format '{{range .RepoDigests}}{{println .}}{{end}}')
  pin=$(printf '%s\n' "$pins" | awk -v prefix="$repo@sha256:" 'index($0,prefix)==1 {print; exit}')
  [[ "$pin" =~ @sha256:[a-f0-9]{64}$ ]] || { echo "REFUSED compute image: no registry digest for $ref" >&2; return 2; }
  printf '%s\n' "$pin"
}

# Resolve actual published components, never silently fall back to a floating tag.
compute_storage=$(immutable_image "$compute_registry/elohim-storage:${STORAGE_TAG:?STORAGE_TAG required}")
compute_doorway=$(immutable_image "$compute_registry/elohim-doorway:${DOORWAY_TAG:?DOORWAY_TAG required}")
compute_relay=$(immutable_image "$compute_registry/iroh-relay:${IROH_RELAY_VERSION:-1.0.3}-${GIT_COMMIT_HASH:?GIT_COMMIT_HASH required}")
compute_beacon=$(immutable_image "$compute_registry/relay-addr-beacon:$GIT_COMMIT_HASH")
bash "$compute_root/scripts/ci/build-compute-worker.sh" --source "$compute_base" "$compute_tools" "$compute_commit"
nerdctl -n k8s.io push "$compute_tools"
compute_tools_pin=$(immutable_image "$compute_tools")
cp "$compute_happ" "$compute_context/elohim.happ"
cp "$compute_root/genesis/agentic/compute/Dockerfile.mesh" "$compute_context/Dockerfile"
node --input-type=module - "$compute_context" "$compute_commit" "$compute_base" "$compute_tools_pin" "$compute_storage" "$compute_doorway" "$compute_relay" "$compute_beacon" <<'JS'
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const [dir, sourceCommit, base, tools, storage, doorway, relay, beacon] = process.argv.slice(2);
const happSha256 = createHash('sha256').update(readFileSync(`${dir}/elohim.happ`)).digest('hex');
writeFileSync(`${dir}/inputs.json`, JSON.stringify({sourceCommit, base, tools, storage, doorway, relay, beacon, happSha256}, null, 2) + '\n');
JS
cp "$compute_context/inputs.json" "$compute_output/inputs.json"
export BUILDKIT_HOST=${BUILDKIT_HOST:-unix:///run/buildkit/buildkitd.sock}
nerdctl -n k8s.io build --build-arg "COMPUTE_TOOLS_IMAGE=$compute_tools_pin" \
  --build-arg "STORAGE_IMAGE=$compute_storage" --build-arg "DOORWAY_IMAGE=$compute_doorway" \
  --build-arg "RELAY_IMAGE=$compute_relay" --build-arg "BEACON_IMAGE=$compute_beacon" \
  --tag "$compute_image" "$compute_context"
nerdctl -n k8s.io push "$compute_image"
compute_pin=$(immutable_image "$compute_image")
node --input-type=module - "$compute_output" "$compute_pin" <<'JS'
import { readFileSync, writeFileSync } from 'node:fs';
const [dir, image] = process.argv.slice(2);
const inputs = JSON.parse(readFileSync(`${dir}/inputs.json`, 'utf8'));
writeFileSync(`${dir}/image.json`, JSON.stringify({image, ...inputs}, null, 2) + '\n');
JS
echo "compute worker published: $compute_pin"
