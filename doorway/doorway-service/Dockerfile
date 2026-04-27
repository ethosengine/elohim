# Dependency stage — cached layer shared by check and build targets
# NOTE: Build context should be repo root directory
# Run: docker build -f doorway/doorway-service/Dockerfile -t doorway .
FROM rust:1.93-slim-bookworm AS deps

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    make \
    && rm -rf /var/lib/apt/lists/*

# Install rustfmt and clippy (not included in slim image)
RUN rustup component add rustfmt clippy

# Copy manifests and local crates first for dependency caching
COPY doorway/doorway-service/Cargo.toml doorway/doorway-service/Cargo.lock ./
COPY crates ./crates
COPY elohim/elohim-compute ./elohim/elohim-compute
COPY elohim/elohim-cache-core ./elohim/elohim-cache-core
COPY elohim/sdk/domains/imagodei/types ./elohim/sdk/domains/imagodei/types
COPY elohim/sdk/domains/infrastructure/types ./elohim/sdk/domains/infrastructure/types
# gate-client + its workspace root and transitive internal members
COPY elohim/Cargo.toml ./elohim/Cargo.toml
COPY elohim/constitution ./elohim/constitution
COPY elohim/elohim-agent/elohim-agent-service ./elohim/elohim-agent/elohim-agent-service
COPY elohim/elohim-agent/gate-client ./elohim/elohim-agent/gate-client
COPY elohim/elohim-agent/gate-types ./elohim/elohim-agent/gate-types
# elohim-agent-service path-deps elohim-agent-defender-specialist (the defender
# stub crate added in dc945de2). The workspace lists it as a member, so cargo
# resolves the path even from the agent-service dependency edge — must be
# copied or the build fails with `failed to read .../specialists/defender/Cargo.toml`.
COPY elohim/elohim-agent/specialists/defender ./elohim/elohim-agent/specialists/defender

# Prune workspace members that aren't copied into the build context.
# Keeps: constitution, elohim-agent-service, gate-client, gate-types,
#   specialists/defender (and elohim-compute via members list).
# Removes: gate-client-zome, eae, elohim-token (not needed for doorway build).
RUN sed -i '/"elohim-agent\/gate-client-zome"/d; /"eae"/d; /"elohim-token"/d' elohim/Cargo.toml

# Create dummy src to build dependencies
RUN mkdir -p src/bin && \
    echo 'fn main() { println!("placeholder"); }' > src/main.rs && \
    echo 'pub fn placeholder() {}' > src/lib.rs && \
    echo 'fn main() { println!("worker placeholder"); }' > src/bin/worker.rs

# Fix the paths to local crates (now relative to /app)
RUN sed -i 's|path = "../../crates/doorway-client"|path = "crates/doorway-client"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/elohim-compute"|path = "elohim/elohim-compute"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/elohim-cache-core"|path = "elohim/elohim-cache-core"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/sdk/domains/imagodei/types"|path = "elohim/sdk/domains/imagodei/types"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/sdk/domains/infrastructure/types"|path = "elohim/sdk/domains/infrastructure/types"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/elohim-agent/gate-client"|path = "elohim/elohim-agent/gate-client"|' Cargo.toml

# Build dependencies (this layer gets cached)
RUN cargo build --release

# Quality check stage — CI quality gate (build with --target check)
FROM deps AS check
RUN rm -rf src target/release/doorway target/release/libdoorway* target/release/deps/doorway* target/release/deps/libdoorway*
COPY doorway/doorway-service/src ./src
COPY doorway/doorway-service/templates ./templates
RUN cargo fmt --check
RUN RUSTFLAGS="" cargo clippy -- -D warnings -W clippy::all
RUN RUSTFLAGS="" cargo test --lib --bins

# Build stage
FROM deps AS builder

# Remove placeholder source and binaries to force rebuild
RUN rm -rf src target/release/doorway target/release/libdoorway* target/release/deps/doorway* target/release/deps/libdoorway*

# Copy actual source and templates (Askama resolves templates at compile time)
COPY doorway/doorway-service/src ./src
COPY doorway/doorway-service/templates ./templates

# Accept git info as build args (build.rs reads these env vars)
ARG GIT_COMMIT_SHORT=unknown
ARG GIT_COMMIT_FULL=unknown
ARG BUILD_TIMESTAMP=unknown

# Touch source files to ensure rebuild and build the release binary
RUN GIT_COMMIT_SHORT=${GIT_COMMIT_SHORT} \
    GIT_COMMIT_FULL=${GIT_COMMIT_FULL} \
    BUILD_TIMESTAMP=${BUILD_TIMESTAMP} \
    touch src/main.rs src/lib.rs && cargo build --release

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
COPY doorway/doorway-service/.env.example /app/.env.example

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
