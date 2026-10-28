#!/usr/bin/env bash
# Jenkins-only assembly recovery under the existing Harbor credential scope.
set -euo pipefail
set +x
umask 077
root=$(git rev-parse --show-toplevel)
# A failed download or login must not leave a previous attempt looking successful.
rm -f "$root/compute-worker-image/image.json" \
  "$root/compute-worker-image/inputs.json" \
  "$root/compute-worker-image/registry-preflight.json"
# Same pinned ORAS version as current Edge hApp-fetch helper. Disposable install.
package_tools=$(mktemp -d /tmp/compute-worker-tools.XXXXXX)
trap 'rm -rf "$package_tools"' EXIT
if ! command -v oras >/dev/null 2>&1; then
  curl -fsSL --max-time 60 https://github.com/oras-project/oras/releases/download/v1.1.0/oras_1.1.0_linux_amd64.tar.gz -o "$package_tools/oras.tar.gz"
  tar -xzf "$package_tools/oras.tar.gz" -C "$package_tools" oras
  export PATH="$package_tools:$PATH"
fi
# BuildKit's authprovider and ORAS read the same short-lived private config.
# No login, image load, pull or inspection touches the node's containerd store.
export DOCKER_CONFIG="$package_tools/docker"
mkdir -m 700 "$DOCKER_CONFIG"
printf '%s' "${HARBOR_PASSWORD:?}" | timeout --signal=TERM --kill-after=5 30 \
  oras login --registry-config "$DOCKER_CONFIG/config.json" \
  harbor.ethosengine.com -u "${HARBOR_USERNAME:?}" --password-stdin
node "$root/genesis/scripts/ci/package-compute-worker.mjs" --inputs "$root/genesis/scripts/ci/edge-1672-inputs.json"
