#!/bin/bash
# Ensure sophia-element assets are available for elohim-app build.
# Sources (in priority order):
#   1. Already in src/assets/sophia-plugin/ (CI pre-populated or previous run)
#   2. Built submodule at sophia/ — copy to assets
#   3. Fetch from Nexus registry via npm pack
#   4. Fetch from GitHub Release via fetch-sophia-release.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
ASSET_DIR="$PROJECT_ROOT/app/elohim-app/src/assets/sophia-plugin"
SOPHIA_DIR="$PROJECT_ROOT/sophia"
UMD_FILE="sophia-element.umd.js"

# --- Source 1: Already in assets (CI or previous local build) ---
if [ -f "$ASSET_DIR/$UMD_FILE" ]; then
  echo "sophia assets already present in $ASSET_DIR"
  verify_widgets=true
else
  mkdir -p "$ASSET_DIR"

  # --- Source 2: Built submodule ---
  if [ -f "$SOPHIA_DIR/packages/sophia-element/dist/$UMD_FILE" ]; then
    echo "Copying sophia artifacts from submodule..."
    cp "$SOPHIA_DIR/packages/sophia-element/dist/$UMD_FILE" "$ASSET_DIR/"
    cp "$SOPHIA_DIR/packages/sophia/dist/index.css" "$ASSET_DIR/" 2>/dev/null || true
    cp "$SOPHIA_DIR/packages/sophia-element/dist/sophia-element.umd.css" "$ASSET_DIR/" 2>/dev/null || true
    verify_widgets=true

  # --- Source 3: Fetch from Nexus ---
  elif command -v npm &>/dev/null; then
    echo "sophia artifacts missing locally. Fetching from Nexus..."
    WORKDIR=$(mktemp -d)
    if cd "$WORKDIR" && \
       npm pack @ethosengine/sophia-element --registry=https://nexus.ethosengine.com/repository/npm/ 2>/dev/null && \
       tar xzf ethosengine-sophia-element-*.tgz; then
      cp package/dist/$UMD_FILE "$ASSET_DIR/"
      cp package/dist/index.css "$ASSET_DIR/" 2>/dev/null || true
      cp package/dist/sophia-element.umd.css "$ASSET_DIR/" 2>/dev/null || true
      echo "Fetched sophia-element from Nexus"
      verify_widgets=true
    fi
    cd "$PROJECT_ROOT"
    rm -rf "$WORKDIR"

  # --- Source 4: GitHub Release fallback ---
  else
    FETCH_SCRIPT="$PROJECT_ROOT/scripts/fetch-sophia-release.sh"
    if [ -x "$FETCH_SCRIPT" ] && command -v gh &>/dev/null; then
      echo "Fetching sophia-element from GitHub Release..."
      "$FETCH_SCRIPT" || true
    fi
  fi
fi

# Create stub CSS files if missing
[ -f "$ASSET_DIR/sophia-theme-overrides.css" ] || \
  echo "/* Sophia theme overrides - Configure via Sophia.configure() API */" > "$ASSET_DIR/sophia-theme-overrides.css"
[ -f "$ASSET_DIR/sophia.css" ] || \
  echo "/* Sophia styles - bundled in UMD */" > "$ASSET_DIR/sophia.css"

# Final check
if [ ! -f "$ASSET_DIR/$UMD_FILE" ]; then
  echo "ERROR: sophia-element.umd.js not available from any source"
  echo "Options:"
  echo "  1. Build from submodule: cd sophia && pnpm install && pnpm build && pnpm build:umd"
  echo "  2. Ensure @ethosengine/sophia-element is published to Nexus"
  exit 1
fi

echo "sophia build artifacts verified"

# Verify expected widgets are bundled in UMD
UMD_PATH="$ASSET_DIR/$UMD_FILE"
REQUIRED_WIDGETS=("likert-scale" "radio" "input-number" "numeric-input" "expression")

if [ "${verify_widgets:-false}" = true ]; then
  for widget in "${REQUIRED_WIDGETS[@]}"; do
    if ! python3 -c "
import sys
data = open('$UMD_PATH').read()
sys.exit(0 if '\"$widget\"' in data else 1)
" 2>/dev/null; then
      echo "ERROR: Widget '$widget' not found in sophia UMD bundle"
      echo "  Ensure it's registered in sophia/packages/sophia/src/basic-widgets.ts"
      exit 1
    fi
  done
  echo "sophia widget coverage verified: ${REQUIRED_WIDGETS[*]}"
fi
