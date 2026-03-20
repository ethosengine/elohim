#!/bin/bash
# Validate sophia submodule is built before elohim-app build
# Can be run from anywhere; finds paths relative to script location

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SOPHIA_DIR="$PROJECT_ROOT/app/sophia"
REQUIRED_FILES=(
  "packages/sophia-element/dist/sophia-element.umd.js"
)

# Check submodule initialized (submodules have a .git file, not directory)
if [ ! -e "$SOPHIA_DIR/.git" ]; then
  echo "ERROR: sophia submodule not initialized"
  echo "Run: git submodule update --init"
  exit 1
fi

# Check required dist files — try fetching from GitHub Release if missing
MISSING=false
for file in "${REQUIRED_FILES[@]}"; do
  if [ ! -f "$SOPHIA_DIR/$file" ]; then
    MISSING=true
    break
  fi
done

if [ "$MISSING" = true ]; then
  echo "sophia artifacts missing locally. Attempting to fetch from GitHub Release..."
  FETCH_SCRIPT="$PROJECT_ROOT/scripts/fetch-sophia-release.sh"
  if [ -x "$FETCH_SCRIPT" ] && command -v gh &>/dev/null; then
    "$FETCH_SCRIPT" || true
  fi

  # Re-check after fetch attempt
  for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$SOPHIA_DIR/$file" ]; then
      echo "ERROR: sophia not built - missing $file"
      echo "Run: cd sophia && pnpm install && pnpm build && pnpm build:umd"
      echo "Or publish a release: ./scripts/sophia-release.sh"
      exit 1
    fi
  done
fi

echo "sophia build artifacts verified"

# Verify expected widgets are bundled in UMD
# Uses python3 because the UMD is a single minified line (3.6MB) that exceeds grep's line handling
UMD_FILE="$SOPHIA_DIR/packages/sophia-element/dist/sophia-element.umd.js"
REQUIRED_WIDGETS=("likert-scale" "radio" "input-number" "numeric-input" "expression")

for widget in "${REQUIRED_WIDGETS[@]}"; do
  if ! python3 -c "
import sys
data = open('$UMD_FILE').read()
sys.exit(0 if '\"$widget\"' in data else 1)
" 2>/dev/null; then
    echo "ERROR: Widget '$widget' not found in sophia UMD bundle"
    echo "  Ensure it's registered in app/sophia/packages/sophia/src/basic-widgets.ts"
    exit 1
  fi
done
echo "sophia widget coverage verified: ${REQUIRED_WIDGETS[*]}"
