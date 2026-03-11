#!/bin/bash
# Build all Elohim DNAs and pack the hApp
# Multi-DNA architecture: lamad + imagodei + infrastructure

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
HC_DIR="$APP_DIR/../holochain"
WORKDIR="$HC_DIR/dna/elohim/workdir"

mkdir -p "$WORKDIR"

echo "🔨 Building Elohim multi-DNA hApp..."

# Build and pack lamad DNA (elohim)
echo "📦 Building lamad DNA..."
cd "$HC_DIR/dna/elohim"
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
hc dna pack . -o "$WORKDIR/lamad.dna"
echo "   ✅ lamad.dna"

# Build and pack imagodei DNA
echo "📦 Building imagodei DNA..."
cd "$HC_DIR/dna/imagodei"
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
hc dna pack . -o "$WORKDIR/imagodei.dna"
echo "   ✅ imagodei.dna"

# Build and pack infrastructure DNA
echo "📦 Building infrastructure DNA..."
cd "$HC_DIR/dna/infrastructure"
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
hc dna pack . -o "$WORKDIR/infrastructure.dna"
echo "   ✅ infrastructure.dna"

# Pack the hApp with all three DNAs
echo "📦 Packing elohim.happ..."
hc app pack "$WORKDIR" -o "$WORKDIR/elohim.happ"

echo ""
echo "✅ Multi-DNA hApp built successfully!"
echo "   Location: $WORKDIR/elohim.happ"
echo "   DNAs: lamad, imagodei, infrastructure"
