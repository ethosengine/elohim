# syntax=docker/dockerfile:1
# ─────────────────────────────────────────────────────────────────────────────
# Custom edgenode carrying the tx5/go-pion zombie-PeerConnection leak fix.
#
# WHAT THIS FIXES (RCA: .claude/data/conductor-leak-rca-tx5-gopion-backpressure-2026-06-17.md):
#   The conductor's off-heap OOM = dead-peer "zombie" PeerConnections never freed,
#   because tx5 0.8.1's go_pion.rs ignored PeerConnectionState changes (Evt::State(_)=>()).
#   Fixed by tx5 #194 + #199 (verified: teardown tests PASS with the fix / TIME OUT without).
#   Also carries holochain #5719 (the receipt-storm amplifier brake).
#
# HOW: holo-host's base downloads a PREBUILT holochain to /bin/holochain. We build OUR
#   patched binary (a6d4e805 + tx5 [patch] #194/#199 + #5719) and swap it in. Pure binary
#   swap — no need to replicate holo-host's whole image build.
#
#   The conductor fork's Cargo.toml [patch.crates-io] routes the tx5 0.8.1 family to ../tx5
#   (our fork). The two forks are cloned side-by-side below so that relative path resolves.
#
# ⚠ TWO THINGS TO VERIFY against holo-host's exact "go-pion-custom" build before fleet use:
#   1. HC_FEATURES — the production feature set (esp. wasmer + any unstable-* flags). The
#      default below is a best-effort starting point derived from the Makefile; holo-host's
#      release recipe is authoritative. A wrong feature set builds a subtly-different binary.
#   2. kitsune2 wire-compat — our binary links kitsune2 0.3.2 (a6d4e805); the live mesh runs
#      kitsune2 0.3.0-dev.3 (holo-host's chore-downgrade branch). Both 0.3.x → expected
#      wire-compatible, but CANARY FIRST (one non-genesis leecher) and confirm gossip/DHT
#      before any genesis-pair or fleet roll (per HANDOFF-2026-06-17-upstream-tx5-transport-pin §4).
#
# BUILD (heavy — CI/build-host: ~20-30 GB target, ~30 min; needs network for crates + Go deps):
#   docker build -f elohim/holochain/edgenode/Dockerfile.zombie-fix \
#     -t <registry>/elohim-edgenode:zombie-fix elohim/holochain/edgenode
# ─────────────────────────────────────────────────────────────────────────────

ARG HC_FORK=https://github.com/ethosengine/holochain.git
ARG HC_BRANCH=elohim-0.6
ARG TX5_FORK=https://github.com/ethosengine/tx5.git
ARG TX5_BRANCH=elohim-0.8.1-zombie-fix
# Best-effort feature set — VERIFY against holo-host's go-pion-custom release recipe.
ARG HC_FEATURES=sqlite-encrypted,wasmer_sys,backend-go-pion

# ── builder: compile the patched glibc holochain binary ──────────────────────
FROM rust:1.91-bookworm AS build
ARG HC_FORK HC_BRANCH TX5_FORK TX5_BRANCH HC_FEATURES
# Go 1.24 + a C toolchain for the tx5-go-pion-sys CGo build (vendored pion/webrtc v4.1.3)
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential cmake perl pkg-config libssl-dev ca-certificates curl git \
 && rm -rf /var/lib/apt/lists/* \
 && curl -fsSL https://go.dev/dl/go1.24.4.linux-amd64.tar.gz | tar -C /usr/local -xz
ENV PATH=/usr/local/go/bin:$PATH \
    CGO_ENABLED=1 \
    RUSTFLAGS="" \
    GOFLAGS=-mod=mod
WORKDIR /src
# clone side-by-side so the conductor's [patch] `../tx5/crates/*` resolves to our fork
RUN git clone --depth 1 -b "${TX5_BRANCH}" "${TX5_FORK}" tx5 \
 && git clone --depth 1 -b "${HC_BRANCH}" "${HC_FORK}" holochain-conductor
WORKDIR /src/holochain-conductor
# NOTE: no --locked (the [patch] updates the lockfile). go-pion backend ONLY (datachannel
# backend pulls datachannel-sys → a C++/openssl/bindgen build we don't want and don't ship).
RUN cargo build --release --bin holochain \
      --no-default-features --features "${HC_FEATURES}"

# ── final: holo-host base, binary swapped ────────────────────────────────────
FROM ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0-go-pion-custom
# /bin/holochain is where the base image's prebuilt conductor lives (entrypoint launches it)
COPY --from=build /src/holochain-conductor/target/release/holochain /bin/holochain
# the elohim conductor config (same as the stock Dockerfile)
COPY conductor-config.yaml /etc/holochain/conductor-config.yaml
ENV CONDUCTOR_MODE=true
EXPOSE 4444 4445
# entrypoint inherited from base: /usr/local/bin/entrypoint.sh → /bin/holochain
