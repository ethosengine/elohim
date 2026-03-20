#!/bin/bash
# Publish sophia-element build artifacts as a GitHub Release.
#
# Usage:
#   ./scripts/sophia-release.sh           # Auto-detect version from sophia/package.json
#   ./scripts/sophia-release.sh 1.2.3     # Explicit version
#
# Prerequisites:
#   - gh CLI authenticated (gh auth login)
#   - sophia built (cd sophia && pnpm build && pnpm build:umd)
#
# Creates release: sophia-v{version} with assets:
#   - sophia-element.umd.js
#   - sophia-element.umd.js.map
#   - sophia-styles.css (base styles from sophia/dist/index.css)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SOPHIA_DIR="$PROJECT_ROOT/sophia"

# Version from arg or sophia-element package.json
VERSION="${1:-$(node -p "require('$SOPHIA_DIR/packages/sophia-element/package.json').version")}"
TAG="sophia-v${VERSION}"

echo "Publishing sophia release: ${TAG}"

# Verify artifacts exist
UMD="$SOPHIA_DIR/packages/sophia-element/dist/sophia-element.umd.js"
UMD_MAP="$SOPHIA_DIR/packages/sophia-element/dist/sophia-element.umd.js.map"
STYLES="$SOPHIA_DIR/packages/sophia/dist/index.css"

for f in "$UMD" "$UMD_MAP" "$STYLES"; do
  if [ ! -f "$f" ]; then
    echo "ERROR: Missing artifact: $f"
    echo "Run: cd sophia && pnpm install && pnpm build && pnpm build:umd"
    exit 1
  fi
done

echo "Artifacts verified:"
echo "  UMD:    $(du -h "$UMD" | cut -f1)"
echo "  Map:    $(du -h "$UMD_MAP" | cut -f1)"
echo "  Styles: $(du -h "$STYLES" | cut -f1)"

# Check if release already exists
if gh release view "$TAG" &>/dev/null; then
  echo "Release ${TAG} already exists. Updating assets..."
  gh release upload "$TAG" \
    "$UMD" \
    "$UMD_MAP" \
    "$STYLES#sophia-styles.css" \
    --clobber
else
  echo "Creating new release ${TAG}..."
  gh release create "$TAG" \
    "$UMD" \
    "$UMD_MAP" \
    "$STYLES#sophia-styles.css" \
    --title "Sophia Element ${VERSION}" \
    --notes "sophia-element UMD bundle + styles for elohim-app consumption.

Assets:
- \`sophia-element.umd.js\` — Web component bundle (load at runtime)
- \`sophia-element.umd.js.map\` — Source map
- \`sophia-styles.css\` — Base Sophia styles

Usage: \`./scripts/fetch-sophia-release.sh\` to download into the app build."
fi

echo "Done: ${TAG}"
