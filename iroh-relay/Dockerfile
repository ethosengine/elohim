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
# --locked IS LOAD-BEARING, not hygiene: iroh-base 0.95.1 exact-pins
# curve25519-dalek =5.0.0-pre.1, the pre-release whose published source
# carries the digest::crypto_common path bug that froze the storage dataplane
# on iroh 0.92. The bug is resolution-conditional; the published lock pins
# the working digest 0.11.0-rc.3. Unlocked is a coin flip.
#
# Version contract + full rationale (four independent lines of evidence for
# the 0.95.1 pairing with conductor 0.6.3 / kitsune2_transport_iroh 0.4.1):
#   genesis/orchestrator/manifests/doorway/alpha.yaml (iroh-relay header,
#   "VERSION CONTRACT" + "HARBOR BUILD RECIPE" sections)
#   genesis/docs/content/elohim-protocol/architecture/2026-08-05-wave2-relay-sovereignty-design.md §2
#
# Build (from this directory):
#   docker build -t harbor.ethosengine.com/ethosengine/iroh-relay:0.95.1-dev-latest .
#   docker build --build-arg IROH_RELAY_VERSION=0.95.1 -t iroh-relay:local .

# ---- builder ---------------------------------------------------------------
FROM rust:1-slim-bookworm AS builder

ARG IROH_RELAY_VERSION=0.95.1

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
