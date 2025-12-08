#!/bin/bash
# Build script for Lamad Spike DNA
#
# Prerequisites:
#   - Rust toolchain with wasm32-unknown-unknown target
#   - Holochain CLI tools (hc)
#
# Usage:
#   ./build.sh

set -e

echo "Building Lamad Spike DNA..."

# Build WASM zomes
echo "Compiling zomes to WASM..."
cargo build --release --target wasm32-unknown-unknown

# Package DNA
echo "Packaging DNA..."
hc dna pack . -o lamad_spike.dna

# Package hApp
echo "Packaging hApp..."
hc app pack workdir -o lamad-spike.happ

echo ""
echo "Build complete!"
echo "  DNA: lamad_spike.dna"
echo "  hApp: lamad-spike.happ"
echo ""
echo "Next steps:"
echo "  1. Start Edge Node: cd ../docker/edge-node && docker compose up -d"
echo "  2. Copy hApp to container: docker cp lamad-spike.happ elohim-edgenode:/tmp/"
echo "  3. Install via browser using HolochainClientService"
