#!/bin/bash
# Generate TypeScript types from Rust Diesel models using ts-rs
#
# Usage:
#   ./scripts/generate-types.sh
#
# Output goes to: holochain/sdk/storage-client-ts/src/generated/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STORAGE_DIR="$(dirname "$SCRIPT_DIR")"
SDK_DIR="$STORAGE_DIR/../sdk/storage-client-ts"
GENERATED_DIR="$SDK_DIR/src/generated"

echo "=== TypeScript Generation from Diesel Models ==="
echo "Storage dir: $STORAGE_DIR"
echo "Output dir:  $GENERATED_DIR"
echo ""

# Ensure output directory exists
mkdir -p "$GENERATED_DIR"

# Run cargo test to trigger ts-rs export
echo "Running cargo test to generate TypeScript bindings..."
cd "$STORAGE_DIR"
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings --release 2>&1 || {
    echo "Note: export_bindings test may not exist yet. Running all tests to generate types..."
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --release 2>&1 || true
}

# Fix ts-rs import paths: ts-rs 10.1 generates imports pointing to
# elohim-storage/bindings/serde_json/JsonValue which is outside the SDK package.
# Rewrite to use the local ./JsonValue copy that ts-rs also generates.
echo ""
echo "Fixing JsonValue import paths..."
find "$GENERATED_DIR" -name '*.ts' -exec sed -i \
    's|from "../../../../elohim-storage/bindings/serde_json/JsonValue"|from "./JsonValue"|g' {} +

# Check what was generated
echo ""
echo "Generated TypeScript files:"
ls -la "$GENERATED_DIR"/*.ts 2>/dev/null || echo "  (no .ts files found yet)"

# Create index.ts that exports all generated types
if [ -d "$GENERATED_DIR" ] && ls "$GENERATED_DIR"/*.ts 1>/dev/null 2>&1; then
    echo ""
    echo "Creating index.ts..."
    cat > "$GENERATED_DIR/index.ts" << 'INDEXEOF'
/**
 * AUTO-GENERATED TypeScript types from Rust Diesel models
 *
 * DO NOT EDIT - regenerate with:
 *   cd holochain/elohim-storage && ./scripts/generate-types.sh
 *
 * Source: holochain/elohim-storage/src/db/models.rs
 */

INDEXEOF

    # Add exports for each generated file
    for file in "$GENERATED_DIR"/*.ts; do
        basename=$(basename "$file" .ts)
        if [ "$basename" != "index" ]; then
            echo "export * from './$basename';" >> "$GENERATED_DIR/index.ts"
        fi
    done

    echo "Generated index.ts with exports for:"
    grep "export \* from" "$GENERATED_DIR/index.ts" | sed 's/.*\.\/\(.*\)./  - \1/'
fi

echo ""
echo "=== Generation complete ==="
