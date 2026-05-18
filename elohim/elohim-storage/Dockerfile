# --- Extract holochain binaries from edgenode image ---
# This copies the conductor binary we've been running in production.
# When we own the conductor version pin, replace this with a direct build.
FROM ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0-go-pion-custom AS conductor-source

# Build stage
# NOTE: Build context should be repo root directory
# Run: docker build -f elohim/elohim-storage/Dockerfile -t elohim-storage .
# Match dev environment Rust version for dependency compatibility
FROM rust:1.94.0-slim-bookworm AS builder

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
COPY elohim/elohim-cache-core ./elohim-cache-core
COPY elohim/elohim-compute ./elohim-compute
COPY elohim/epr ./epr
# Plan 2 SDK boundary: elohim-views holds wire-shape Views; elohim-storage
# depends on it directly. Without this COPY the build aborts at resolve time:
# "failed to read /elohim-views/Cargo.toml: No such file".
COPY elohim/elohim-views ./elohim-views

# Fix the paths to local crates (now relative to /app)
RUN sed -i 's|path = "../../crates/doorway-client"|path = "crates/doorway-client"|' Cargo.toml && \
    sed -i 's|path = "../constitution"|path = "constitution"|' Cargo.toml && \
    sed -i 's|path = "../elohim-cache-core"|path = "elohim-cache-core"|' Cargo.toml && \
    sed -i 's|path = "../elohim-compute"|path = "elohim-compute"|' Cargo.toml && \
    sed -i 's|path = "../elohim-views"|path = "elohim-views"|' Cargo.toml && \
    sed -i 's|path = "../epr"|path = "epr"|' Cargo.toml

# Strip the optional `elohim-render` path-dep + the `ssr` feature that
# selects it. elohim-render is the V8/deno_core SSR runtime — it lives
# in the doorway image, not here. The `ssr` feature is opt-in and not
# enabled in this build, but cargo still resolves the path at metadata
# time (and `dep:elohim-render` in the feature list would also error if
# the optional dep is missing). Storage source code uses
# `#[cfg(feature = "ssr")]` to gate ssr-only paths — those just stay
# inactive in this image, which is the intended deployment shape.
RUN sed -i \
    -e '/^elohim-render = .*optional = true.*$/d' \
    -e '/^ssr = \["dep:elohim-render"\]$/d' \
    Cargo.toml

# Constitution crate uses workspace inheritance — inline the values since
# the workspace root (elohim/Cargo.toml) isn't copied into the build context
RUN sed -i \
    -e 's|version.workspace = true|version = "0.1.0"|' \
    -e 's|edition.workspace = true|edition = "2021"|' \
    -e 's|license.workspace = true|license = "Apache-2.0"|' \
    -e 's|authors.workspace = true|authors = ["Elohim Protocol"]|' \
    -e 's|tokio.workspace = true|tokio = { version = "1.43", features = ["full", "sync"] }|' \
    -e 's|async-trait.workspace = true|async-trait = "0.1"|' \
    -e 's|serde.workspace = true|serde = { version = "1.0", features = ["derive"] }|' \
    -e 's|serde_json.workspace = true|serde_json = "1.0"|' \
    -e 's|serde_yaml.workspace = true|serde_yaml = "0.9"|' \
    -e 's|sha2.workspace = true|sha2 = "0.10"|' \
    -e 's|hex.workspace = true|hex = "0.4"|' \
    -e 's|thiserror.workspace = true|thiserror = "2.0"|' \
    -e 's|tracing.workspace = true|tracing = "0.1"|' \
    -e 's|chrono.workspace = true|chrono = { version = "0.4", features = ["serde"] }|' \
    -e 's|uuid.workspace = true|uuid = { version = "1.16", features = ["v4", "serde"] }|' \
    constitution/Cargo.toml

# elohim-epr crate uses workspace inheritance too — inline the values.
# Unlike constitution, epr uses inline-feature forms for serde and ts-rs.
RUN sed -i \
    -e 's|version.workspace = true|version = "0.1.0"|' \
    -e 's|edition.workspace = true|edition = "2021"|' \
    -e 's|serde = { workspace = true, features = \["derive"\] }|serde = { version = "1.0", features = ["derive"] }|' \
    -e 's|serde_json.workspace = true|serde_json = "1.0"|' \
    -e 's|chrono.workspace = true|chrono = { version = "0.4", features = ["serde"] }|' \
    -e 's|thiserror.workspace = true|thiserror = "2.0"|' \
    -e 's|ts-rs = { workspace = true, features = \["chrono-impl", "serde-compat"\] }|ts-rs = { version = "10.0", features = ["chrono-impl", "serde-compat"] }|' \
    -e 's|hex.workspace = true|hex = "0.4"|' \
    epr/Cargo.toml

# Create dummy src + bench placeholders to build dependencies.
# Any `[[bin]]`, `[[bench]]`, or `[[example]]` entry declared in Cargo.toml
# requires a matching file at manifest-parse time; the dep-cache stage builds
# before the real sources are COPY'd, so we satisfy every declared target with
# an empty placeholder. The cleanup step below removes them all before the
# real COPY, so this layer only ever caches third-party crates.
RUN mkdir -p src benches && \
    echo 'fn main() { println!("placeholder"); }' > src/main.rs && \
    echo 'pub fn placeholder() {}' > src/lib.rs && \
    echo 'fn main() {}' > benches/graph_traversal.rs

# Build dependencies (this layer gets cached)
RUN RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release

# Remove placeholder source, binaries, and proc-macro outputs to force rebuild.
# CRITICAL: build/ and .fingerprint/ contain embed_migrations! output from the
# placeholder build (empty migrations dir). Without removing these, Cargo reuses
# the cached proc-macro result and the real migrations are never embedded.
RUN rm -rf src benches target/release/elohim-storage target/release/libelohim* \
    target/release/deps/elohim* target/release/deps/libelohim* \
    target/release/build/diesel* target/release/.fingerprint/diesel* \
    target/release/build/elohim* target/release/.fingerprint/elohim*

# Cache buster — pass --build-arg CACHE_BUST=$(git rev-parse HEAD) to invalidate
# when source changes but Docker thinks COPY layers are identical
ARG CACHE_BUST=unknown
ARG GIT_COMMIT_SHORT=unknown
ARG GIT_COMMIT_FULL=unknown
ARG BUILD_TIMESTAMP=unknown
ARG RUSTC_VERSION=unknown

# Copy actual source, migrations, and benches (every declared Cargo target
# must have its files present at manifest-parse time; missing benches/ aborts
# `cargo build` before the binary even compiles).
# Migrations must be present for diesel embed_migrations! proc macro.
COPY elohim/elohim-storage/migrations ./migrations
COPY elohim/elohim-storage/src ./src
COPY elohim/elohim-storage/benches ./benches

# bootstrap_manifests.rs uses include_str!("../../../sdk/schemas/v1/manifests/...")
# which resolves to /sdk/... in this WORKDIR /app layout (../../.. from
# /app/src/services/ lands at /). Copying elohim/sdk to /sdk satisfies that
# compile-time path. Without this, cargo build fails with "couldn't read
# bootstrap-{standing,tending}-policy.json: No such file or directory".
COPY elohim/sdk /sdk

# Build the release binary
# CACHE_BUST is interpolated into the command so BuildKit treats it as a new layer
# when the value changes (ARG alone is insufficient for BuildKit cache invalidation)
RUN echo "Building at ${CACHE_BUST}" && \
    RUSTFLAGS='--cfg getrandom_backend="custom"' \
    GIT_COMMIT_SHORT=${GIT_COMMIT_SHORT} \
    GIT_COMMIT_FULL=${GIT_COMMIT_FULL} \
    BUILD_TIMESTAMP=${BUILD_TIMESTAMP} \
    RUSTC_VERSION=${RUSTC_VERSION} \
    cargo build --release

# Verify migrations are actually embedded in the binary.
# If embed_migrations! ran against an empty dir, none of our CREATE TABLE statements appear.
RUN strings target/release/elohim-storage | grep -q "CREATE TABLE content" \
    || (echo "FATAL: embed_migrations! did not pick up migrations. Binary has no schema embedded." && exit 1)

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

# Conductor data directory (lair keystore, chain state)
RUN mkdir -p /var/local/lib/holochain && chown storage:storage /var/local/lib/holochain

# Copy binary from builder
COPY --from=builder /app/target/release/elohim-storage /usr/local/bin/elohim-storage

# Holochain conductor binary — for embedded conductor mode.
# Extracted from the same edgenode image we've been running in production.
COPY --from=conductor-source /usr/bin/holochain /usr/local/bin/holochain

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
