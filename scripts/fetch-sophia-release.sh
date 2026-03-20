#!/bin/bash
# Fetch sophia-element artifacts from the latest GitHub Release.
#
# Usage:
#   ./scripts/fetch-sophia-release.sh             # Latest release
#   ./scripts/fetch-sophia-release.sh sophia-v1.2.3  # Specific tag
#
# Prerequisites:
#   - gh CLI authenticated (gh auth login)
#
# Downloads to the same paths the local build produces, so angular.json
# asset paths work unchanged. Falls back to local build if present.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SOPHIA_DIR="$PROJECT_ROOT/sophia"

TAG="${1:-}"

# Target directories (match sophia build output paths)
ELEMENT_DIST="$SOPHIA_DIR/packages/sophia-element/dist"
SOPHIA_DIST="$SOPHIA_DIR/packages/sophia/dist"

# Check if artifacts already exist (local build takes precedence)
if [ -f "$ELEMENT_DIST/sophia-element.umd.js" ] && [ -f "$SOPHIA_DIST/index.css" ]; then
  echo "sophia artifacts already present (local build). Skipping download."
  exit 0
fi

# Ensure target directories exist
mkdir -p "$ELEMENT_DIST" "$SOPHIA_DIST"

# Determine which release to fetch
if [ -z "$TAG" ]; then
  TAG=$(gh release list --limit 1 --json tagName --jq '.[0].tagName' 2>/dev/null | grep '^sophia-v' || true)
  if [ -z "$TAG" ]; then
    echo "ERROR: No sophia release found. Run: ./scripts/sophia-release.sh"
    exit 1
  fi
fi

echo "Fetching sophia artifacts from release: ${TAG}"

# Download assets
gh release download "$TAG" \
  --pattern "sophia-element.umd.js" \
  --dir "$ELEMENT_DIST" \
  --clobber

gh release download "$TAG" \
  --pattern "sophia-element.umd.js.map" \
  --dir "$ELEMENT_DIST" \
  --clobber

gh release download "$TAG" \
  --pattern "sophia-styles.css" \
  --dir "$SOPHIA_DIST" \
  --output "$SOPHIA_DIST/index.css" \
  --clobber

echo "sophia artifacts downloaded:"
echo "  UMD:    $(du -h "$ELEMENT_DIST/sophia-element.umd.js" | cut -f1)"
echo "  Styles: $(du -h "$SOPHIA_DIST/index.css" | cut -f1)"
echo "Done."
