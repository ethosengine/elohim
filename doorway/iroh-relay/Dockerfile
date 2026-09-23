# iroh-relay — stock crates.io iroh-relay server, packaged for the doorway
# relay plane (the tx5/SBD signal function's successor for the iroh transport
# generation — see genesis/orchestrator/manifests/doorway/alpha.yaml's
# iroh-relay header).
#
# Self-contained: build context is THIS directory only. Not a fork, not a
# repo-internal crate — this Dockerfile `cargo install`s the published
# iroh-relay crate. crates.io is consumed directly, deliberately NOT mirrored
# through Nexus (matches relay-addr-beacon/Dockerfile's 2026-07-30 decision:
# Nexus is for first-party components only).
#
# --locked: build exactly the crate's published resolution (at 1.0.3 the
# crypto chain is stable — curve25519-dalek 5.0.0 / ed25519-dalek 3.0.0 /
# digest 0.11.3 — so the 0.95.1-era pre-release-dalek hazard is gone, but a
# relay is the one component whose job is being boring; keep it pinned).
#
# Version contract (holochain 0.7.0 era, 2026-09-03): conductor 0.7.0 ->
# kitsune2 0.5.0 -> kitsune2_transport_iroh 0.5.0 -> STOCK iroh 1.0.3 ->
# iroh-relay 1.0.3 client. The server is the same crate at the same version.
# History + rationale:
#   genesis/orchestrator/manifests/doorway/alpha.yaml (iroh-relay header,
#   "VERSION CONTRACT" + "HARBOR BUILD RECIPE" sections)
#   genesis/docs/content/elohim-protocol/architecture/2026-08-05-wave2-relay-sovereignty-design.md §2
#
# Build (from this directory):
#   docker build -t harbor.ethosengine.com/ethosengine/iroh-relay:1.0.3-dev-latest .
#   docker build --build-arg IROH_RELAY_VERSION=1.0.3 -t iroh-relay:local .

# ---- builder ---------------------------------------------------------------
FROM rust:1-slim-bookworm AS builder

ARG IROH_RELAY_VERSION=1.0.3

# pkg-config is required by iroh-relay's transitive dep graph.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Native build — clear the Holochain WASM getrandom flag if it leaks in.
ENV RUSTFLAGS=""

RUN cargo install iroh-relay --version ${IROH_RELAY_VERSION} --locked \
    --features server --root /out

# ---- runtime ---------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# ca-certificates for TLS (the relay's own outbound calls, if any).
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /out/bin/iroh-relay /usr/local/bin/iroh-relay

ENTRYPOINT ["/usr/local/bin/iroh-relay"]
