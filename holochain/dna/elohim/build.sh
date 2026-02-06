#!/bin/bash
# Build script for Elohim hApp (multi-DNA)
#
# Prerequisites:
#   - Rust toolchain with wasm32-unknown-unknown target
#   - Holochain CLI tools (hc)
#
# Usage:
#   ./build.sh

set -e

echo "Building Elohim hApp..."

# Build WASM zomes for lamad DNA
echo "Compiling lamad zomes to WASM..."
cargo build --release --target wasm32-unknown-unknown

# Package lamad DNA
echo "Packaging lamad DNA..."
hc dna pack . -o workdir/lamad.dna

# Build infrastructure DNA
echo "Compiling infrastructure zomes..."
(cd ../infrastructure && cargo build --release --target wasm32-unknown-unknown)
hc dna pack ../infrastructure -o workdir/infrastructure.dna

# Build imagodei DNA
echo "Compiling imagodei zomes..."
(cd ../imagodei && cargo build --release --target wasm32-unknown-unknown)
hc dna pack ../imagodei -o workdir/imagodei.dna

# Package hApp
echo "Packaging hApp..."
hc app pack workdir -o elohim.happ

echo ""
echo "Build complete!"
echo "  DNAs: workdir/lamad.dna, workdir/infrastructure.dna, workdir/imagodei.dna"
echo "  hApp: elohim.happ"
echo ""
echo "Next steps:"
echo "  1. Start Edge Node: cd ../docker/edge-node && docker compose up -d"
echo "  2. Copy hApp to container: docker cp elohim.happ elohim-edgenode:/tmp/"
echo "  3. Install via browser using HolochainClientService"
