# Build stage
# NOTE: Build context should be repo root directory
# Run: docker build -f doorway/Dockerfile -t doorway .
FROM rust:1.83-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests and local crates first for dependency caching
COPY doorway/Cargo.toml doorway/Cargo.lock ./
COPY holochain/crates ./holochain/crates
COPY holochain/holochain-cache-core ./holochain/holochain-cache-core

# Create dummy src to build dependencies
RUN mkdir -p src/bin && \
    echo 'fn main() { println!("placeholder"); }' > src/main.rs && \
    echo 'pub fn placeholder() {}' > src/lib.rs && \
    echo 'fn main() { println!("worker placeholder"); }' > src/bin/worker.rs

# Fix the paths to local crates (now relative to /app)
RUN sed -i 's|path = "../holochain/crates/doorway-client"|path = "holochain/crates/doorway-client"|' Cargo.toml && \
    sed -i 's|path = "../holochain/holochain-cache-core"|path = "holochain/holochain-cache-core"|' Cargo.toml

# Build dependencies (this layer gets cached)
RUN cargo build --release

# Remove placeholder source and binaries to force rebuild
RUN rm -rf src target/release/doorway target/release/libdoorway* target/release/deps/doorway* target/release/deps/libdoorway*

# Copy actual source
COPY doorway/src ./src

# Touch source files to ensure rebuild and build the release binary
RUN touch src/main.rs src/lib.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 doorway

# Copy binary from builder
COPY --from=builder /app/target/release/doorway /usr/local/bin/doorway

# Copy default config
COPY doorway/.env.example /app/.env.example

# Set ownership
RUN chown -R doorway:doorway /app

USER doorway

# Default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run doorway
ENTRYPOINT ["doorway"]
CMD ["--listen", "0.0.0.0:8080"]
