#!/bin/bash
# Validate sophia submodule is built before elohim-app build
# Can be run from anywhere; finds paths relative to script location

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SOPHIA_DIR="$PROJECT_ROOT/sophia"
REQUIRED_FILES=(
  "packages/sophia-element/dist/sophia-element.umd.js"
)

# Check submodule initialized (submodules have a .git file, not directory)
if [ ! -e "$SOPHIA_DIR/.git" ]; then
  echo "ERROR: sophia submodule not initialized"
  echo "Run: git submodule update --init"
  exit 1
fi

# Check required dist files
for file in "${REQUIRED_FILES[@]}"; do
  if [ ! -f "$SOPHIA_DIR/$file" ]; then
    echo "ERROR: sophia not built - missing $file"
    echo "Run: cd sophia && pnpm install && pnpm build && pnpm build:umd"
    exit 1
  fi
done

echo "sophia build artifacts verified"
