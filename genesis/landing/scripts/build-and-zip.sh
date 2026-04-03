#!/usr/bin/env bash
# genesis/landing/scripts/build-and-zip.sh
#
# Build the landing page, ZIP the dist, and compute the SHA256 hash.
# Output: dist/protocol-landing.zip and dist/protocol-landing.sha256

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "=== Building landing page ==="
npx tsx scripts/render-content.ts
npx vite build

echo "=== Creating ZIP ==="
cd dist
zip -r ../dist/protocol-landing.zip . -x "protocol-landing.zip" "protocol-landing.sha256"
cd ..

echo "=== Computing SHA256 ==="
sha256sum dist/protocol-landing.zip | awk '{print $1}' > dist/protocol-landing.sha256
HASH=$(cat dist/protocol-landing.sha256)

echo ""
echo "Build complete:"
echo "  ZIP:    dist/protocol-landing.zip"
echo "  SHA256: $HASH"
echo "  Size:   $(du -h dist/protocol-landing.zip | awk '{print $1}')"
