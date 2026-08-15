#!/usr/bin/env bash
# Install dependencies for the facade-based substrate-verify runner
# (genesis/a2o/scripts/substrate-verify.ts) and build @elohim/storage-client,
# whose dist/ the runner imports. Runs before the first substrate-validation
# stage (Verify Peer Mesh); the E2E stage's later full install subsumes this.
#
# Filtered install (package + its workspace deps) keeps the substrate stages
# lean — the full ~2.7k-package workspace install stays where it always was,
# in E2E Verification.
set -euo pipefail

cd "$(dirname "$0")/../../.."  # repo/workspace root

echo "Installing substrate-verify runner dependencies (filtered: a2o + storage-client)..."
pnpm install --frozen-lockfile --filter '@elohim/a2o...' --filter '@elohim/storage-client...' 2>/dev/null \
  || pnpm install --filter '@elohim/a2o...' --filter '@elohim/storage-client...'

echo "Building @elohim/storage-client..."
pnpm --filter @elohim/storage-client build

echo "✅ substrate-verify runner ready"
