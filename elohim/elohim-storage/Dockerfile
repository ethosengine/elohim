# --- Extract holochain binaries from edgenode image ---
# This copies the conductor binary embedded into elohim-storage (embedded-conductor mode).
# CONDUCTOR_SOURCE_IMAGE = the jemalloc-PRODUCTION conductor — jemalloc as the global allocator
# with C-side malloc interposition (unprefixed_malloc_on_supported_platforms), profiling OFF.
# This is THE cure for the conductor's native glibc-arena heap leak (OOM ~8GB/~5h): jemalloc's
# decay/munmap returns the freed pages glibc pinned in chained 64MB secondary arenas.
# Built by che-devworkspaces jenkins/Jenkinsfile-elohim-edgenode (HC_FEATURES includes `jemalloc`;
# the isolated -prof and -iroh variant images can never satisfy the production pin).
#
# WHICH conductor is decided by the BUILD, not by this line. CI passes
# --build-arg CONDUCTOR_SOURCE_IMAGE=…:conductor-<sha12>, where <sha12> is the commit
# this repo's `elohim/holochain-conductor` submodule pointer records
# (scripts/ci/build-storage-image.sh). The elohim-conductor pipeline publishes that
# tag from the same pointer, so bumping the submodule is the whole act of moving a
# conductor onto the fleet — see elohim/conductor-image/README.md.
#
# The default below is only for LOCAL/manual `docker build` without CI's build-arg. It
# is a moving tag and is NOT what the fleet gets. CI never falls back to it: if the
# pinned conductor tag is absent from Harbor the build FAILS with a remediation
# message, because a storage image whose conductor doesn't match its committed source
# is precisely the failure this pin exists to prevent.
#
# ⚠ ROLLBACK: do NOT roll back to a glibc conductor — ghcr.io/holo-host/edgenode (stock) and any
# pre-jemalloc edgenode build carry the heap leak. Roll back by moving the submodule pointer to
# the previous jemalloc conductor commit; the pin tag follows it automatically.
# (History: this ARG was a hand-edited sha256 digest, bumped by a human after each edgenode
# build — which meant a rebuilt conductor silently did NOT reach the fleet, and a GC'd digest
# 404'd the whole edge build with no diagnosis (edge #1261, 2026-07-29). The tx5
# zombie-PeerConnection theory (#194/#199) and go-pion were the WRONG RCA — deployed fleet-wide
# and the leak persisted. The 2026-06-18 jemalloc-PROF debug build localized it to glibc malloc
# arenas; the allocator swap is the fix. Details + the superseded-RCA trail:
# .claude/data/conductor-leak-jemalloc-prod-changeset-2026-06-19.md.)
ARG CONDUCTOR_SOURCE_IMAGE=harbor.ethosengine.com/ethosengine/elohim-edgenode:latest
FROM ${CONDUCTOR_SOURCE_IMAGE} AS conductor-source

# Build stage
# NOTE: Build context should be repo root directory
# Run: docker build -f elohim/elohim-storage/Dockerfile -t elohim-storage .
# Match dev environment Rust version for dependency compatibility
# `deps` pre-builds the dependency graph against placeholder sources; the
# `check` (CI quality gate, `--target check` only) and `builder` (release)
# stages both branch off it, so a source-only change never recompiles
# third-party crates in either path.
FROM rust:1.94.0-slim-bookworm AS deps

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

# rustfmt for the `check` stage's fmt gate. Lives HERE (right after apt, before
# any COPY) so the layer caches until the base-image pin bumps — placed later
# it would re-download from static.rust-lang.org on every path-dep source
# change (frequent), making upstream flake a spurious gate failure.
RUN rustup component add rustfmt

# Copy manifests and local crates first for dependency caching.
# `COPY crates ./crates` is wholesale, so crates/seam-contracts (the
# elohim-seam-contracts leaf crate, a path-dep of elohim-storage since the
# 2026-08-02 seam-concern adoption) rides along with no separate COPY. It
# declares its OWN `[workspace]` table (that is what gives it its own Cargo.lock,
# which its `no_heavy_deps_in_dep_tree` boundary test reads via include_str!) —
# cargo treats such a path-dep as a separate workspace root rather than
# auto-membering it, so the flattened /app layout needs no `[workspace]` strip
# and no workspace-field inline (the crate declares every field literally).
# Only the relative path needs rewriting, in the sed block below.
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
# elohim-facings: the pure lens crate (select→fold→aggregate folds over the
# holder-relation); a path-dep of elohim-storage since the 2026-06-19 facings
# extraction. Without this COPY the build aborts at resolve time:
# "failed to read /elohim-facings/Cargo.toml: No such file". (Its own
# `../elohim-views` dep resolves correctly in the flattened /app layout — no rewrite.)
COPY elohim/elohim-facings ./elohim-facings
# elohim-peer-fabric: std-only path-dep (storage serve-routing feature). Flattened /app layout;
# sed-rewrite below. Without this COPY the build aborts: "failed to read /elohim-peer-fabric/Cargo.toml".
COPY elohim/elohim-peer-fabric ./elohim-peer-fabric
# elohim-render: optional path-dep (Cargo.toml: optional = true) gated by the `ssr`
# feature. Cargo resolves path-deps at metadata time even when the feature is inactive,
# so the Cargo.toml must point at a real directory or the resolve aborts. The default
# build does NOT enable ssr (lean, no V8); pass --build-arg STORAGE_FEATURES=ssr to
# produce the SSR-enabled variant (app-serving nodes). Both share this COPY layer.
COPY elohim/elohim-render ./elohim-render
# elohim-chrome-asset: non-optional path-dep — storage serves the runtime-served
# EPR omnibar client element from the sidecar (/chrome route). Cargo resolves
# path-deps at metadata time regardless of features, so the directory must exist
# or the build aborts at resolve: "failed to read /elohim-chrome-asset/Cargo.toml".
COPY elohim/elohim-chrome-asset ./elohim-chrome-asset
# Wave 3 — valueflows bridge crates (hREA / VF-GraphQL interop) are
# unconditional path-deps in Cargo.toml; copy them in so the
# build-context contains their source.
COPY bridges ./bridges

# Substrate-rea-replication-fix (2026-05-26): elohim-storage's
# conductor_writes facade depends on `shefa-types` and `lamad-types` as
# `path = "../sdk/domains/<dna>/types"`. From /app/Cargo.toml's perspective
# that's `/sdk/...`. The full elohim/sdk tree is COPYd to /sdk further
# down for the bootstrap_manifests include_str! paths, but cargo
# resolves Cargo.toml path-deps at the dep-cache stage above — so the
# two type crates MUST be present BEFORE line 130's first `cargo build`.
# Without these COPYs the build aborts:
#   error: failed to read `/sdk/domains/shefa/types/Cargo.toml`
#   Caused by: No such file or directory (os error 2)
# (This was the silent-deploy-failure that stalled edge #1007 onwards
# until this Dockerfile fix.)
COPY elohim/sdk/domains/shefa/types /sdk/domains/shefa/types
COPY elohim/sdk/domains/lamad/types /sdk/domains/lamad/types

# Admin-seam unblock (2026-08-07): Cargo.toml's `[patch.crates-io]` points
# holochain_serialized_bytes at `path = "../../vendor/holochain_serialized_bytes-0.0.56"`.
# From /app/Cargo.toml that normalises to `/vendor/...` — same trick as `/sdk`
# above. A `[patch]` path is resolved during dependency RESOLUTION, so like the
# type crates it MUST exist before the first `cargo build` in this stage, or the
# build aborts with:
#   error: failed to read `/vendor/holochain_serialized_bytes-0.0.56/Cargo.toml`
# See vendor/holochain_serialized_bytes-0.0.56/VENDOR-NOTE.md for why it exists.
COPY vendor /vendor

# Fix the paths to local crates (now relative to /app)
RUN sed -i 's|path = "../../crates/doorway-client"|path = "crates/doorway-client"|' Cargo.toml && \
    sed -i 's|path = "../../crates/elohim-sdk"|path = "crates/elohim-sdk"|' Cargo.toml && \
    sed -i 's|path = "../../crates/seam-contracts"|path = "crates/seam-contracts"|' Cargo.toml && \
    sed -i 's|path = "../constitution"|path = "constitution"|' Cargo.toml && \
    sed -i 's|path = "../elohim-cache-core"|path = "elohim-cache-core"|' Cargo.toml && \
    sed -i 's|path = "../elohim-compute"|path = "elohim-compute"|' Cargo.toml && \
    sed -i 's|path = "../elohim-views"|path = "elohim-views"|' Cargo.toml && \
    sed -i 's|path = "../elohim-facings"|path = "elohim-facings"|' Cargo.toml && \
    sed -i 's|path = "../elohim-peer-fabric"|path = "elohim-peer-fabric"|' Cargo.toml && \
    sed -i 's|path = "../elohim-render"|path = "elohim-render"|' Cargo.toml && \
    sed -i 's|path = "../elohim-chrome-asset"|path = "elohim-chrome-asset"|' Cargo.toml && \
    sed -i 's|path = "../epr"|path = "epr"|' Cargo.toml && \
    sed -i 's|path = "../../bridges/valueflows/valueflows-bridge"|path = "bridges/valueflows/valueflows-bridge"|' Cargo.toml && \
    sed -i 's|path = "../../bridges/valueflows/valueflows-types"|path = "bridges/valueflows/valueflows-types"|' Cargo.toml && \
    sed -i 's|path = "../../bridges/did/did-bridge"|path = "bridges/did/did-bridge"|' Cargo.toml && \
    sed -i 's|path = "../../bridges/did/did-types"|path = "bridges/did/did-types"|' Cargo.toml

# Crates under ./crates reference elohim-views via `../../elohim/elohim-views`
# (the source-tree layout: crates/<name>/ → ../../elohim/elohim-views/). In the
# container, elohim-views is COPYd to /app/elohim-views (without the nested
# `elohim/` segment), so the relative path resolves to a nonexistent dir and
# cargo aborts with "failed to read /app/elohim/elohim-views/Cargo.toml — No
# such file or directory". Rewrite the path in every crates/<name>/Cargo.toml
# that references elohim-views to drop the extra segment.
RUN sed -i 's|path = "../../elohim/elohim-views"|path = "../../elohim-views"|' \
        crates/elohim-sdk/Cargo.toml \
        crates/elohim-storage-client/Cargo.toml


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
    -e 's|ts-rs = { workspace = true, optional = true }|ts-rs = { version = "10.0", optional = true }|' \
    -e 's|tokio-test = { workspace = true }|tokio-test = "0.4"|' \
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

# elohim-compute crate uses workspace inheritance too — inline the values.
RUN sed -i \
    -e 's|serde = { workspace = true }|serde = { version = "1.0", features = ["derive"] }|' \
    -e 's|serde_json = { workspace = true }|serde_json = "1.0"|' \
    -e 's|dashmap = { workspace = true }|dashmap = "6.1"|' \
    -e 's|chrono = { workspace = true }|chrono = { version = "0.4", features = ["serde"] }|' \
    elohim-compute/Cargo.toml

# elohim-render crate uses workspace inheritance too — inline the values.
# Added when storage gained the optional `ssr` feature (elohim-render path-dep):
# the workspace root (elohim/Cargo.toml) isn't in the build context, so cargo
# fails "error inheriting `edition` from workspace root ... failed to find a
# workspace root" at metadata time — even for the lean (no-ssr) build, since
# cargo resolves the optional dep's manifest regardless. Same fix as constitution/epr.
RUN sed -i \
    -e 's|version.workspace = true|version = "0.1.0"|' \
    -e 's|edition.workspace = true|edition = "2021"|' \
    -e 's|license.workspace = true|license = "Apache-2.0"|' \
    -e 's|serde.workspace = true|serde = { version = "1.0", features = ["derive"] }|' \
    -e 's|serde_json.workspace = true|serde_json = "1.0"|' \
    -e 's|sha2.workspace = true|sha2 = "0.10"|' \
    -e 's|thiserror.workspace = true|thiserror = "2.0"|' \
    -e 's|tracing.workspace = true|tracing = "0.1"|' \
    -e 's|async-trait.workspace = true|async-trait = "0.1"|' \
    -e 's|tokio = { workspace = true }|tokio = { version = "1.43", features = ["full", "sync"] }|' \
    -e 's|deno_core.workspace = true|deno_core = "0.339.0"|' \
    -e 's|deno_error.workspace = true|deno_error = "0.5.7"|' \
    elohim-render/Cargo.toml

# elohim-chrome-asset uses workspace inheritance too — inline the values since
# the workspace root (elohim/Cargo.toml) isn't in the build context. Only the
# fields its manifest actually inherits: version/edition/license + sha2 +
# serde/serde_json (ChromeContext, 2026-07-11 — missing these two broke edge
# #1174 at dep-resolve: "failed to find a workspace root").
RUN sed -i \
    -e 's|version.workspace = true|version = "0.1.0"|' \
    -e 's|edition.workspace = true|edition = "2021"|' \
    -e 's|license.workspace = true|license = "Apache-2.0"|' \
    -e 's|sha2.workspace = true|sha2 = "0.10"|' \
    -e 's|serde.workspace = true|serde = { version = "1.0", features = ["derive"] }|' \
    -e 's|serde_json.workspace = true|serde_json = "1.0"|' \
    elohim-chrome-asset/Cargo.toml

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

# Build dependencies (this layer gets cached).
# Mirror the real build's `--features p2p-iroh` (below) so the iroh / iroh-blobs
# / iroh-gossip dependency crates land in THIS cached layer instead of
# recompiling cold in the real-source stage. The placeholder src/ has no
# p2p_iroh module — the feature only activates the optional dep entries here.
RUN RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --features p2p-iroh

# Remove placeholder source, binaries, and proc-macro outputs to force rebuild.
# CRITICAL: build/ and .fingerprint/ contain embed_migrations! output from the
# placeholder build (empty migrations dir). Without removing these, Cargo reuses
# the cached proc-macro result and the real migrations are never embedded.
RUN rm -rf src benches target/release/elohim-storage target/release/libelohim* \
    target/release/deps/elohim* target/release/deps/libelohim* \
    target/release/build/diesel* target/release/.fingerprint/diesel* \
    target/release/build/elohim* target/release/.fingerprint/elohim*

# ═══════════════════════════════════════════════════════════════════════════
# check-deps: dev-profile dependency warm for the quality gate — branched OFF
# the release path so the default build (deps → builder → runtime) never pays
# for it (the deps cache chain includes every path-dep source tree, so it
# churns far more often than "manifest changes"). Requires BuildKit: a legacy
# builder executes ALL stages in file order — CI uses buildkitd; local one-off
# builds need DOCKER_BUILDKIT=1.
# ═══════════════════════════════════════════════════════════════════════════
FROM deps AS check-deps
# Recreate the placeholder targets (deps removed them after the release warm).
RUN mkdir -p src benches && \
    echo 'fn main() { println!("placeholder"); }' > src/main.rs && \
    echo 'pub fn placeholder() {}' > src/lib.rs && \
    echo 'fn main() {}' > benches/graph_traversal.rs
# Debug-info trim — mirrors the repo root .cargo/config.toml policy (validated
# 2026-06-04, −57% target size), which is NOT copied into the image: without
# this the test build emits full DWARF (multi-GB layers, heavier links).
RUN mkdir -p .cargo && printf '[profile.dev]\ndebug = "line-tables-only"\n[profile.dev.package."*"]\ndebug = false\n' > .cargo/config.toml
# Warm the dev+test dependency graph against placeholder src: without this,
# every quality-gate run cold-compiles the full graph.
RUN RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --no-run
# Same placeholder-poisoning rm as the release path, dev-profile dirs — the
# check stage's test build must never reuse a placeholder-built
# embed_migrations! output (empty migrations → false green).
RUN rm -rf src benches target/debug/elohim-storage target/debug/libelohim* \
    target/debug/deps/elohim* target/debug/deps/libelohim* \
    target/debug/build/diesel* target/debug/.fingerprint/diesel* \
    target/debug/build/elohim* target/debug/.fingerprint/elohim*

# ═══════════════════════════════════════════════════════════════════════════
# Quality gate (CI `--target check`) — the CI home for the storage lib suite.
# The release image build alone proves "compiles", NOT "passes": storage-logic
# regressions rode green edge builds until 2026-07-01 (1835 lib tests incl.
# the CRDT sync/heal proofs had no CI home). Mirrors doorway's `check` target:
# built ONLY via `--target check`; the default build path never descends here.
# Keep this stage's COPY block in sync with builder's.
# Deliberately test+fmt only (no clippy): clippy's check-profile dep compile
# is a third full graph; lint debt is covered by the pre-push heavy gates.
# Deliberately NO CACHE_BUST: COPY layers key on content hashes, so an
# unchanged tree is a full cache hit and a changed tree re-runs exactly once.
# ═══════════════════════════════════════════════════════════════════════════
FROM check-deps AS check
COPY elohim/elohim-storage/migrations ./migrations
COPY elohim/elohim-storage/src ./src
COPY elohim/elohim-storage/benches ./benches
# config/: cfg(test) include_str! targets (policy/config.rs reads
# ../../config/peer-policy.example.toml) — test-only, so the release path
# never needed it and edge #1137's first test-layer run failed compile on it.
COPY elohim/elohim-storage/config ./config
# bridges/did/schemas: same class — services/did_identity_store.rs cfg(test)
# include_str!("../../../../bridges/did/schemas/did-document-1.1.schema.json")
# clamps to the container-absolute /bridges/did/schemas from /app/src/services,
# so the release path never needed it but cargo test --lib does (mirrors
# doorway/doorway-service/Dockerfile). Kept UNSTABLE since the Wave C1 DID work.
COPY bridges/did/schemas /bridges/did/schemas
COPY elohim/sdk /sdk
RUN cargo fmt --check
RUN CARGO_INCREMENTAL=0 RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib

# ---------------------------------------------------------------------------
# SDK feature-matrix gate (CI `--target sdk-check` only) — never on the
# default build path. Same pattern as `check`: the CI builder image carries NO
# Rust toolchain (by design — see scripts/ci/storage-quality-gate.sh), so the
# matrix runs in here. Deliberately NOT `FROM deps`: crates/elohim-sdk is its
# own workspace root whose `../../elohim/elohim-views` path deps expect the
# REPO shape, not the flattened /app layout — and an independent stage keeps
# its layer cache keyed on crates/** + elohim-views alone. The script it runs
# is the SAME file the local pre-push gate runs (single source of matrix
# truth); inside this stage cargo is on PATH so the script takes its direct
# path.
FROM rust:1.94.0-slim-bookworm AS sdk-check
# libsqlite3-dev: the SDK's `native` feature links SYSTEM sqlite via
# rusqlite→libsqlite3-sys (no `bundled` feature) — present in the dev
# container, absent from slim-bookworm.
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy
WORKDIR /sdk-gate
COPY crates ./crates
COPY elohim/elohim-views ./elohim/elohim-views
COPY scripts/ci/elohim-sdk-feature-matrix.sh ./scripts/ci/elohim-sdk-feature-matrix.sh
RUN bash scripts/ci/elohim-sdk-feature-matrix.sh

# ═══════════════════════════════════════════════════════════════════════════
# Release build stage (the default image path: deps → builder → runtime).
# ═══════════════════════════════════════════════════════════════════════════
FROM deps AS builder

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

# Build the release binary.
# CACHE_BUST is interpolated into the command so BuildKit treats it as a new layer
# when the value changes (ARG alone is insufficient for BuildKit cache invalidation).
#
# `p2p-iroh` is ALWAYS built into the image (not a cargo default feature) so the
# co-resident dual-stack (`TransportBackend::Dual`, activated at runtime via
# `ELOHIM_TRANSPORT_BACKEND=dual`) ships. Default features (`p2p`, `graph-native`)
# stay on, so the image carries both transport stacks; the runtime picks which to
# run. STORAGE_FEATURES ADDS optional cargo features on top of p2p-iroh:
#   default (empty) — p2p-iroh only (lean, no V8; suitable for all nodes)
#   ssr             — p2p-iroh + elohim-render + V8; for app-serving nodes that
#                     materialize and serve the Angular SSR bundle from the substrate
# Pass --build-arg STORAGE_FEATURES=ssr to CI/operator to produce the SSR variant
# (it becomes `--features p2p-iroh,ssr`).
ARG STORAGE_FEATURES=""
RUN echo "Building at ${CACHE_BUST}" && \
    RUSTFLAGS='--cfg getrandom_backend="custom"' \
    GIT_COMMIT_SHORT=${GIT_COMMIT_SHORT} \
    GIT_COMMIT_FULL=${GIT_COMMIT_FULL} \
    BUILD_TIMESTAMP=${BUILD_TIMESTAMP} \
    RUSTC_VERSION=${RUSTC_VERSION} \
    cargo build --release --features "p2p-iroh${STORAGE_FEATURES:+,$STORAGE_FEATURES}"

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
