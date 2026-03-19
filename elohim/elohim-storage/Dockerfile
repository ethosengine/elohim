# Build stage
# NOTE: Build context should be repo root directory
# Run: docker build -f elohim/elohim-storage/Dockerfile -t elohim-storage .
# Note: sysinfo 0.37.2 (via lair_keystore 0.6.3) requires Rust 1.88+
FROM rust:1.88-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
# - build-essential: provides make, gcc for native builds (libsodium-sys-stable)
# - libsodium-dev: prebuilt libsodium to avoid compiling from source
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    libsodium-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests and local crates first for dependency caching
COPY elohim/elohim-storage/Cargo.toml elohim/elohim-storage/Cargo.lock ./
COPY crates ./crates
COPY elohim/constitution ./constitution

# Fix the paths to local crates (now relative to /app)
RUN sed -i 's|path = "../../crates/doorway-client"|path = "crates/doorway-client"|' Cargo.toml && \
    sed -i 's|path = "../constitution"|path = "constitution"|' Cargo.toml

# Create dummy src to build dependencies
RUN mkdir -p src && \
    echo 'fn main() { println!("placeholder"); }' > src/main.rs && \
    echo 'pub fn placeholder() {}' > src/lib.rs

# Build dependencies (this layer gets cached)
RUN cargo build --release

# Remove placeholder source and binaries to force rebuild
RUN rm -rf src target/release/elohim-storage target/release/libelohim* \
    target/release/deps/elohim* target/release/deps/libelohim*

# Copy actual source
COPY elohim/elohim-storage/src ./src

# Touch source files to ensure rebuild and build the release binary
RUN touch src/main.rs src/lib.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsodium23 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 storage

# Create data directory
RUN mkdir -p /data && chown storage:storage /data

# Copy binary from builder
COPY --from=builder /app/target/release/elohim-storage /usr/local/bin/elohim-storage

# Set ownership
RUN chown -R storage:storage /app

USER storage

# Default ports:
# 8090 - HTTP API (blobs, import queue)
EXPOSE 8090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8090/health || exit 1

# Run elohim-storage
ENTRYPOINT ["elohim-storage"]
CMD ["--http-port", "8090", "--storage-dir", "/data"]
