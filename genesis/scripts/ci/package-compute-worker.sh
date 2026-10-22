#!/usr/bin/env bash
# Jenkins-only assembly recovery under the existing Harbor credential scope.
set -euo pipefail
set +x
root=$(git rev-parse --show-toplevel)
printf '%s' "${HARBOR_PASSWORD:?}" | nerdctl -n k8s.io login harbor.ethosengine.com -u "${HARBOR_USERNAME:?}" --password-stdin
# Same pinned ORAS version as current Edge hApp-fetch helper. Disposable install.
package_tools=$(mktemp -d)
trap 'rm -rf "$package_tools"' EXIT
if ! command -v oras >/dev/null 2>&1; then
  curl -fsSL https://github.com/oras-project/oras/releases/download/v1.1.0/oras_1.1.0_linux_amd64.tar.gz -o "$package_tools/oras.tar.gz"
  tar -xzf "$package_tools/oras.tar.gz" -C "$package_tools" oras
  export PATH="$package_tools:$PATH"
fi
printf '%s' "$HARBOR_PASSWORD" | oras login harbor.ethosengine.com -u "$HARBOR_USERNAME" --password-stdin
node "$root/genesis/scripts/ci/package-compute-worker.mjs" --inputs "$root/genesis/scripts/ci/edge-1672-inputs.json"
