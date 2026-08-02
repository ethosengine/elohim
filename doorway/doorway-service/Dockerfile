# Dependency stage — cached layer shared by check and build targets
# NOTE: Build context should be repo root directory
# Run: docker build -f doorway/doorway-service/Dockerfile -t doorway .
FROM rust:1.93-slim-bookworm AS deps

WORKDIR /app

# Install build dependencies.
# curl + ca-certificates are needed by the v8 crate's build.rs (pulled in via
# elohim-render → deno_core), which downloads a ~190MB prebuilt librusty_v8
# from github.com/denoland/rusty_v8/releases. The build script falls back to
# curl when its Python downloader fails; the slim image has neither, so v8
# build.rs panics with `Result::unwrap()` on a NotFound IO error.
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    make \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install rustfmt and clippy (not included in slim image)
RUN rustup component add rustfmt clippy

# Copy manifests and local crates first for dependency caching.
# `COPY crates ./crates` is wholesale, so crates/seam-contracts (the
# elohim-seam-contracts leaf crate, a path-dep of doorway-service since the
# 2026-08-02 seam-concern adoption in render/registry.rs) rides along with no
# separate COPY. It declares its OWN `[workspace]` table, which cargo treats as a
# separate workspace root rather than auto-membering into /app's — so no
# `[workspace]` strip and no workspace-field inline are needed (the crate
# declares every field literally). Only the relative path needs rewriting, in
# the sed block below.
COPY doorway/doorway-service/Cargo.toml doorway/doorway-service/Cargo.lock ./
COPY crates ./crates
COPY elohim/elohim-compute ./elohim/elohim-compute
COPY elohim/elohim-cache-core ./elohim/elohim-cache-core
COPY elohim/sdk/domains/imagodei/types ./elohim/sdk/domains/imagodei/types
COPY elohim/sdk/domains/infrastructure/types ./elohim/sdk/domains/infrastructure/types
# Plan 2 SDK boundary: View wire-shapes live in elohim-views. doorway-service
# consumes it directly; without this COPY the build aborts at resolve time:
# "failed to read /elohim/elohim-views/Cargo.toml: No such file".
COPY elohim/elohim-views ./elohim/elohim-views
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
# elohim-render is a non-optional path-dep of doorway-service (Cargo.toml line ~129)
# and a workspace member of elohim/Cargo.toml. Without this COPY, cargo aborts at
# resolve time: "failed to read /elohim/elohim-render/Cargo.toml: No such file".
COPY elohim/elohim-render ./elohim/elohim-render
# elohim-chrome-asset: TRANSITIVE path-dep — elohim-render depends on it
# (elohim-render/Cargo.toml: path = "../elohim-chrome-asset"), and it's a
# workspace member of elohim/Cargo.toml. doorway-service doesn't name it directly,
# but cargo must read /app/elohim/elohim-chrome-asset/Cargo.toml when resolving
# render. Without this COPY the build aborts at resolve time: "failed to read
# /app/elohim/elohim-chrome-asset/Cargo.toml: No such file". Inherits the
# workspace root (elohim/Cargo.toml, COPYd above) like render — no sed/inline.
COPY elohim/elohim-chrome-asset ./elohim/elohim-chrome-asset
# elohim-peer-fabric: std-only path-dep (doorway membrane policy, edge-defense feature) and a
# workspace member of elohim/Cargo.toml. Without this COPY cargo aborts at resolve time:
# "failed to read /elohim/elohim-peer-fabric/Cargo.toml: No such file".
COPY elohim/elohim-peer-fabric ./elohim/elohim-peer-fabric
# bridges/did: did-types + did-bridge are path-deps of doorway-service (DID
# resolution surfaces). They inherit from the bridges/did workspace root
# (version.workspace = true, serde = { workspace = true }, …), so cargo must read
# bridges/did/Cargo.toml when resolving them — COPY the workspace root + both
# crates. The root's `members` lists did-tests (not copied); prune it below or
# cargo aborts: "failed to load manifest for workspace member did-tests".
COPY bridges/did/Cargo.toml ./bridges/did/Cargo.toml
COPY bridges/did/did-types ./bridges/did/did-types
COPY bridges/did/did-bridge ./bridges/did/did-bridge
RUN sed -i '/"did-tests"/d' bridges/did/Cargo.toml

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
    sed -i 's|path = "../../crates/seam-contracts"|path = "crates/seam-contracts"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/elohim-compute"|path = "elohim/elohim-compute"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/elohim-cache-core"|path = "elohim/elohim-cache-core"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/sdk/domains/imagodei/types"|path = "elohim/sdk/domains/imagodei/types"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/sdk/domains/infrastructure/types"|path = "elohim/sdk/domains/infrastructure/types"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/elohim-agent/gate-client"|path = "elohim/elohim-agent/gate-client"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/elohim-views"|path = "elohim/elohim-views"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/elohim-render"|path = "elohim/elohim-render"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/elohim-peer-fabric"|path = "elohim/elohim-peer-fabric"|' Cargo.toml && \
    sed -i 's|path = "../../bridges/did/did-types"|path = "bridges/did/did-types"|' Cargo.toml && \
    sed -i 's|path = "../../bridges/did/did-bridge"|path = "bridges/did/did-bridge"|' Cargo.toml

# Build dependencies (this layer gets cached)
RUN cargo build --release

# Quality check stage — CI quality gate (build with --target check)
FROM deps AS check
RUN rm -rf src target/release/doorway target/release/libdoorway* target/release/deps/doorway* target/release/deps/libdoorway*
COPY doorway/doorway-service/src ./src
COPY doorway/doorway-service/templates ./templates
# Shared test-vector fixtures consumed via `include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
# "/../../elohim/sdk/fixtures/..."))` in #[cfg(test)] modules (epr_router.rs, server/http.rs).
# CARGO_MANIFEST_DIR is /app, so the path resolves to /elohim/sdk/fixtures. These are
# test-only — `cargo build --release` (deps/builder stages) never touches them, so the gap
# only surfaces here at `cargo test --lib --bins`. Without this COPY the test build aborts:
# "couldn't read .../route-claims.vectors.json: No such file or directory (os error 2)".
COPY elohim/sdk/fixtures /elohim/sdk/fixtures
# DID 1.1 conformance schema — consumed via include_str! in routes/identity.rs
# #[cfg(test)] to validate the emitted did:web document. CARGO_MANIFEST_DIR is
# /app here, so the concat!(.../../../bridges/did/schemas/...) path resolves to
# /bridges/did/schemas. Test-only (like the fixtures above) — the release build
# never reads it, so the gap only surfaces at `cargo test --lib --bins`.
COPY bridges/did/schemas /bridges/did/schemas
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

# SSR bundle — materialized at boot from the substrate (no longer baked into this image).
# The doorway process fetches the bundle content-addressed from the DHT and writes it
# into /opt/elohim-render at startup. SSR_BUNDLE_PATH=/opt/elohim-render/main.server.mjs
# tells elohim-render::AngularRenderer where to find the entry point.
# init_renderer in server/http.rs:239 silently returns None if the bundle fails to load,
# so the pod boots cleanly even if the substrate bundle is not yet available.

# Copy default config
COPY doorway/doorway-service/.env.example /app/.env.example

# Ensure /opt/elohim-render exists and is owned by the doorway user so the
# runtime-materialize step (and/or an emptyDir mount) can write the bundle at boot.
RUN mkdir -p /opt/elohim-render && chown -R doorway:doorway /app /opt/elohim-render

USER doorway

# Default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run doorway
ENTRYPOINT ["doorway"]
CMD ["--listen", "0.0.0.0:8080"]
