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

# Copy manifests and local crates first for dependency caching
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
    sed -i 's|path = "../../elohim/elohim-agent/gate-client"|path = "elohim/elohim-agent/gate-client"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/elohim-views"|path = "elohim/elohim-views"|' Cargo.toml && \
    sed -i 's|path = "../../elohim/elohim-render"|path = "elohim/elohim-render"|' Cargo.toml

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

# SSR bundle stage — builds elohim-app's Angular server bundle for in-process SSR.
# Independent of the Rust stages above; BuildKit parallelizes them.
# Bundle is ~50MB. Reference: genesis/docs/superpowers/specs/2026-05-07-doorway-ssr-runtime-design.md
# §Build-side change to elohim-app (line 229-242).
#
# This stage exists in the doorway Dockerfile (rather than as a separate
# Harbor artifact + initContainer) to avoid cross-pipeline tag coordination
# between elohim-app and elohim-edge. Tradeoff: ~5-10 minutes added to
# doorway image build on a cold cache, plus pnpm/Node toolchain in the
# build phase. The followup project_doorway_full_facilitator_sprint memory
# entry covers retiring this redundancy by absorbing the SPA host into
# doorway entirely.
FROM node:22-alpine AS ssr-bundle
WORKDIR /workspace

# Tooling: native-dep build (some npm packages have C++ bindings) + bash for
# any prebuild scripts ng build's pipeline reaches for.
RUN apk add --no-cache python3 make g++ git bash curl
# Pin pnpm to the version declared in root package.json's `packageManager`
# field. Mismatched pnpm versions hash patches/@angular__build@19.2.22.patch
# differently, tripping ERR_PNPM_LOCKFILE_CONFIG_MISMATCH on
# --frozen-lockfile. Keep this in sync with package.json.
RUN npm install -g pnpm@10.30.3

# Workspace metadata — pnpm-lock.yaml requires every workspace member's
# package.json to exist for --frozen-lockfile to pass.
COPY pnpm-workspace.yaml pnpm-lock.yaml package.json ./

# Hoist all deps to the workspace root so ng build can find tslib (and
# every other transitively-resolved package) regardless of which workspace
# package owns the import. Mirrors the repo-root .npmrc — that file isn't
# COPY'd because it carries the Nexus registry URL and an `${NPM_TOKEN}`
# auth placeholder that wouldn't expand inside the docker build context.
# We only need the hoist + peer-deps behavior here; default public npm is
# fine because pnpm install --frozen-lockfile pulls from the URLs already
# resolved in pnpm-lock.yaml.
RUN printf 'shamefully-hoist=true\nlegacy-peer-deps=true\nstrict-peer-dependencies=false\n' > .npmrc

# pnpm patchedDependencies — root package.json declares
# `"@angular/build@19.2.22": "patches/@angular__build@19.2.22.patch"`.
# Without this dir, `pnpm install --frozen-lockfile` aborts with
# "ENOENT: no such file or directory, open '/workspace/patches/...'".
COPY patches ./patches

# Copy workspace package directories. Source files for elohim-app's
# transitive workspace deps (elohim-core, storage-client-ts) are pulled in
# here; other dirs are copied for their package.json metadata only —
# pnpm install with --filter scopes won't traverse their src.
COPY app ./app
COPY elohim/sdk ./elohim/sdk
COPY elohim/elohim-agent ./elohim/elohim-agent
COPY doorway/doorway-app ./doorway/doorway-app
COPY steward/device ./steward/device
COPY genesis/seeder ./genesis/seeder
COPY genesis/a2o ./genesis/a2o
COPY genesis/orchestrator ./genesis/orchestrator
COPY genesis/landing ./genesis/landing

# elohim-app declares `"elohim-cache-core": "file:../../elohim/elohim-cache-core/pkg"`
# (a wasm-pack output dir whose .wasm/.js files are gitignored — only
# package.json is tracked). pnpm install --frozen-lockfile aborts with
# "ENOENT: scandir '/workspace/elohim/elohim-cache-core/pkg'" without it.
# The wasm is browser-loaded at runtime via dynamic import; the SSR server
# bundle never references it, so copying just the tracked package.json is
# enough to satisfy the lockfile + the angular.json assets entry.
COPY elohim/elohim-cache-core/pkg ./elohim/elohim-cache-core/pkg

# Install elohim-app + its transitive workspace deps. We also need
# `@elohim/service` (elohim-library/projects/elohim-service) — it's NOT
# a package.json workspace dep of elohim-app (no `"@elohim/service":
# "workspace:*"` entry); it's pulled in via tsconfig path aliases
# (`"@elohim/service": ["../elohim-library/projects/elohim-service/src/
# index.ts"]`). When ng build resolves those path-aliased imports, the
# library's own peerDeps (@angular/core, rxjs, tslib...) need to be
# installed under app/elohim-library/projects/elohim-service/node_modules.
RUN pnpm install --filter "elohim-app..." --filter "@elohim/service..." --frozen-lockfile

# Stub prebuild outputs so ng build doesn't fail on missing browser-only
# artifacts (service worker, wasm cache core, fonts). The server bundle
# never references these — they're browser-side performance enhancements.
RUN mkdir -p app/elohim-app/src/assets/fonts && \
    : > app/elohim-app/src/assets/apps-sw.js

# Build the workspace TypeScript deps that elohim-app's ng build resolves
# at compile time. Their dist/ directories are gitignored, and we bypass
# the elohim-app `prebuild` hook (which fetches fonts, builds wasm, and
# verifies the sophia submodule — none needed for the server bundle), so
# we have to build these explicitly. Mirrors the elohim-app pipeline's
# build order in the root Jenkinsfile (storage-client first, then core).
RUN pnpm --filter @elohim/storage-client run build && \
    pnpm --filter elohim-core run build

# Build the server bundle. We invoke ng directly to bypass the `prebuild`
# hook. The `postbuild` hook (the Node-shim script) is run explicitly
# afterward.
WORKDIR /workspace/app/elohim-app
RUN pnpm exec ng build --configuration=alpha
RUN node scripts/ssr-shim-node.mjs

# Result lives at /workspace/app/elohim-app/dist/elohim-app/server/.
# Contains: main.server.mjs (entry), chunk-*.mjs (code-split bundles),
# assets-chunks/, node-shims/ (the 22 Node built-in shims the postbuild
# script writes), polyfills.server.mjs.

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

# SSR bundle — read at startup by elohim-render::AngularRenderer when
# SSR_BUNDLE_PATH=/opt/elohim-render/main.server.mjs is set on the pod.
# init_renderer in server/http.rs:239 silently returns None if the bundle
# fails to load, so the pod boots cleanly even if this stage produced an
# unusable bundle (e.g., a future Angular/SSR API mismatch).
COPY --from=ssr-bundle /workspace/app/elohim-app/dist/elohim-app/server /opt/elohim-render

# Copy default config
COPY doorway/doorway-service/.env.example /app/.env.example

# Set ownership
RUN chown -R doorway:doorway /app /opt/elohim-render

USER doorway

# Default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run doorway
ENTRYPOINT ["doorway"]
CMD ["--listen", "0.0.0.0:8080"]
